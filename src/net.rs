// /src/net.rs
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub fn http_get(host: &str, port: u16, path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect((host, port))?;
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(Duration::from_secs(15)))?;

    let req = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: bb_scraper_stdonly/0.2\r\nConnection: close\r\n\r\n",
        path, host
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp = String::from_utf8_lossy(&buf);

    let status_line_end = resp.find("\r\n").unwrap_or(0);
    let status = &resp[..status_line_end];
    if !status.contains("200") {
        return Err(format!("HTTP error: {}", status).into());
    }

    let body_idx = resp.find("\r\n\r\n").ok_or("Malformed HTTP response")? + 4;
    Ok(resp[body_idx..].to_string())
}
