use std::io::{Read, Write};
use std::net::TcpListener;

/// 试点最小后端：只暴露 /health，供 CI/E2E 健康检查。
/// 故意零依赖——业务逻辑由后续迭代通过 issue→oh 长出来。
fn respond_to(request_line: &str) -> (&'static str, &'static str) {
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
    match path {
        "/health" => ("200 OK", r#"{"status":"ok"}"#),
        _ => ("404 Not Found", r#"{"status":"not_found"}"#),
    }
}

fn handle(mut stream: std::net::TcpStream) {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).unwrap_or(0);
    let request_line = String::from_utf8_lossy(&buf[..n])
        .lines()
        .next()
        .unwrap_or("")
        .to_string();
    let (status, body) = respond_to(&request_line);
    let resp = format!(
        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        body.len(),
        body
    );
    let _ = stream.write_all(resp.as_bytes());
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8080").expect("bind 8080");
    println!("pilot-backend listening on :8080");
    for stream in listener.incoming().flatten() {
        handle(stream);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_returns_ok() {
        let (status, body) = respond_to("GET /health HTTP/1.1");
        assert_eq!(status, "200 OK");
        assert!(body.contains("ok"));
    }

    #[test]
    fn unknown_path_returns_404() {
        let (status, _) = respond_to("GET /nope HTTP/1.1");
        assert_eq!(status, "404 Not Found");
    }
}
