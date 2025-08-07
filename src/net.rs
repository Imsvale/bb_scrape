// /src/net.rs
// Very minimal HTTP GET over plain TCP, no TLS.
// Uses HTTP/1.0 so the server closes the connection at the end (no chunked transfer).

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

/// Perform a plain HTTP GET request and return the response body as a String.
///
/// * `host` – hostname (no protocol, no port)
/// * `port` – usually 80 for HTTP
/// * `path` – path + query string starting with `/`
///
/// This function:
/// 1. Connects via TCP.
/// 2. Sends a simple HTTP/1.0 GET request with `Connection: close`.
/// 3. Reads until EOF.
/// 4. Checks for a 200 status line.
/// 5. Returns the body after the header section.
pub fn http_get(host: &str, port: u16, path: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Connect and set reasonable timeouts
    let mut stream = TcpStream::connect((host, port))?;
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(Duration::from_secs(15)))?;

    // Send GET request
    let req = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: bb_scraper_stdonly/0.2\r\nConnection: close\r\n\r\n",
        path, host
    );
    stream.write_all(req.as_bytes())?;
    stream.flush()?;

    // Read the entire response
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    let resp = String::from_utf8_lossy(&buf);

    // Basic status check
    let status_line_end = resp.find("\r\n").unwrap_or(0);
    let status = &resp[..status_line_end];
    if !status.contains("200") {
        return Err(format!("HTTP error: {}", status).into());
    }

    // Split off the body
    let body_idx = resp.find("\r\n\r\n").ok_or("Malformed HTTP response")? + 4;
    Ok(resp[body_idx..].to_string())
}
