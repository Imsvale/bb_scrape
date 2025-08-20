// src/core/net.rs
#![allow(unused)]

// HTTP/1.0 GET over TCP (std-only)

use std::{io::{Read, Write}, net::TcpStream, time::Duration};
use crate::params::{HOST, PREFIX};

pub fn http_get(path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut s = TcpStream::connect((HOST, 80))?;
    s.set_read_timeout(Some(Duration::from_secs(15)))?;
    s.set_write_timeout(Some(Duration::from_secs(15)))?;

    let full = format!("{}{}", PREFIX, path);
    let req = format!(
        "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: bb_scrape/0.4\r\nConnection: close\r\n\r\n",
        full, HOST
    );
    s.write_all(req.as_bytes())?;
    s.flush()?;

    let mut buf = Vec::new();
    s.read_to_end(&mut buf)?;
    let resp = String::from_utf8_lossy(&buf);

    let status = resp.split("\r\n").next().unwrap_or("");
    if !status.contains("200") {
        return Err(format!("HTTP error: {} {}{}", status, HOST, full).into());
    }
    let body_idx = resp.find("\r\n\r\n").ok_or("Malformed HTTP response")? + 4;
    Ok(resp[body_idx..].to_string())
}
