// src/core/net.rs

// HTTP/1.1 GET over TCP (std-only)

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    time::{Duration, Instant},
};
use crate::config::consts::{HOST, PREFIX};

fn join_prefix_and_path(prefix: &str, path: &str) -> String {
    let pfx = prefix.trim_end_matches('/');
    let pth = path.trim_start_matches('/');
    format!("{}/{}", pfx, pth)
}

pub fn http_get(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let full = join_prefix_and_path(PREFIX, path);
    logd!("HTTP GET → {}{}", HOST, &full);

    let t0 = Instant::now();

    // 1) Connect
    let t_connect0 = Instant::now();
    let mut s = TcpStream::connect((HOST, 80))?;
    s.set_read_timeout(Some(Duration::from_secs(15)))?;
    s.set_write_timeout(Some(Duration::from_secs(15)))?;
    let dt_connect = t_connect0.elapsed();
    logd!("HTTP GET · connected in {:?}", dt_connect);

    // 2) Send request
    let t_write0 = Instant::now();
    let req = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: bb_scrape/0.4\r\nConnection: close\r\nAccept-Encoding: identity\r\n\r\n",
        full, HOST
    );
    s.write_all(req.as_bytes())?;
    s.flush()?;
    let dt_write = t_write0.elapsed();
    logd!("HTTP GET · request sent in {:?}", dt_write);

    // 3) Read headers (measure TTFB and header time separately)
    let mut br = BufReader::new(s);

    // read headers line-by-line until CRLFCRLF; time first byte separately
    let t_read0 = Instant::now();
    let mut header_buf: Vec<u8> = Vec::with_capacity(2048);
    let mut first_byte_at: Option<Instant> = None;

    loop {
        let mut line = String::new();
        // blocking read of a line
        let n = br.read_line(&mut line)?;
        if first_byte_at.is_none() && n > 0 {
            first_byte_at = Some(Instant::now());
            logd!("HTTP GET · first byte after {:?}", t_read0.elapsed());
        }
        if n == 0 {
            return Err("EOF before headers complete".into());
        }
        header_buf.extend_from_slice(line.as_bytes());
        if header_buf.ends_with(b"\r\n\r\n") || header_buf.ends_with(b"\n\n") {
            break;
        }
    }

    let t_headers_done = Instant::now();
    let dt_ttfb   = first_byte_at.map(|t| t.duration_since(t_read0)).unwrap_or_default();
    let dt_hdrs   = t_headers_done.duration_since(first_byte_at.unwrap_or(t_read0));
    logd!("HTTP GET · headers read in {:?}", dt_hdrs);

    // 4) Parse status + headers
    let headers = String::from_utf8_lossy(&header_buf);
    let mut lines = headers.split("\r\n").filter(|l| !l.is_empty());
    let status = lines.next().unwrap_or("");
    if !status.contains("200") {
        loge!("HTTP GET · status not OK: {}", status);
        return Err(format!("HTTP error: {} {}{}", status, HOST, full).into());
    }

    let mut content_length: Option<usize> = None;
    let mut chunked = false;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(v) = lower.strip_prefix("content-length:") {
            content_length = v.trim().parse::<usize>().ok();
        } else if lower.starts_with("transfer-encoding:") && lower.contains("chunked") {
            chunked = true;
        }
    }
    logd!(
        "HTTP GET · status OK; content-length={:?}; chunked={}",
        content_length,
        chunked
    );

    // 5) Read body (don’t wait for close when CL is present)
    let t_body0 = Instant::now();
    let mut body: Vec<u8> = Vec::new();

    if let Some(len) = content_length {
        body.reserve_exact(len);
        // read_exact on the underlying reader after the headers already consumed
        let mut take = br.take(len as u64);
        let _ = take.read_to_end(&mut body)?;
        if body.len() != len {
            loge!(
                "HTTP GET · short read: expected {} bytes, got {}",
                len,
                body.len()
            );
        }
    } else if chunked {
        loop {
            let mut size_line = String::new();
            br.read_line(&mut size_line)?;
            if size_line.is_empty() { break; }
            let size_hex = size_line.trim();
            let size = usize::from_str_radix(size_hex, 16).unwrap_or(0);
            if size == 0 {
                // Consume trailing CRLF after 0-size chunk
                let mut _crlf = [0u8; 2];
                let _ = br.read_exact(&mut _crlf);
                break;
            }
            let mut chunk = vec![0u8; size];
            br.read_exact(&mut chunk)?;
            body.extend_from_slice(&chunk);
            let mut _crlf = [0u8; 2];
            let _ = br.read_exact(&mut _crlf);
        }
    } else {
        br.read_to_end(&mut body)?;
    }

    let dt_body = t_body0.elapsed();
    let total = t0.elapsed();
    let kbps = if dt_body.as_secs_f64() > 0.0 {
        (body.len() as f64 / 1024.0) / dt_body.as_secs_f64()
    } else { 0.0 };

    logd!("HTTP GET · body {} bytes in {:?} (~{:.1} KiB/s)", body.len(), dt_body, kbps);
    logd!("HTTP GET · TTFB {:?}", dt_ttfb);
    logd!("HTTP GET ← done total {:?}", total);

    Ok(String::from_utf8_lossy(&body).into_owned())
}

#[cfg(test)]
mod tests {
    use super::join_prefix_and_path;

    #[test]
    fn join_handles_slashes() {
        assert_eq!(join_prefix_and_path("/brutalball", "team.php?i=1"),
                   "/brutalball/team.php?i=1");
        assert_eq!(join_prefix_and_path("/brutalball/", "team.php?i=1"),
                   "/brutalball/team.php?i=1");
        assert_eq!(join_prefix_and_path("/brutalball", "/team.php?i=1"),
                   "/brutalball/team.php?i=1");
        assert_eq!(join_prefix_and_path("/brutalball/", "/team.php?i=1"),
                   "/brutalball/team.php?i=1");
    }
}

