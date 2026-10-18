use crate::queue::JobQueue;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

const STATUS_PAGE: &str = include_str!("status.html");

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        content_type,
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

// A GET-only, headers-ignored, single-request-per-connection HTTP server --
// just enough to serve a status page and the JSON it polls. Not meant to
// face anything but a browser (or curl) on a trusted network.
fn handle(mut stream: TcpStream, queue: &Arc<Mutex<JobQueue>>) {
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });

    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    // Drain and ignore headers up to the blank line that ends them.
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => return,
            Ok(_) if line == "\r\n" || line == "\n" => break,
            Ok(_) => {}
            Err(_) => return,
        }
    }

    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();

    match path.as_str() {
        "/" => respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            STATUS_PAGE,
        ),
        "/api/jobs" => {
            let snapshot = queue.lock().expect("JobQueue lock poisoned").snapshot();
            let json = serde_json::to_string(&snapshot).expect("JobSnapshot always serialises");
            respond(&mut stream, "200 OK", "application/json", &json);
        }
        _ => respond(&mut stream, "404 Not Found", "text/plain", "not found"),
    }
}

// Runs forever, accepting one connection at a time on its own thread.
pub fn serve(queue: Arc<Mutex<JobQueue>>, port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port))?;
    println!("Status page at http://localhost:{}/", port);

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let queue = queue.clone();
        thread::spawn(move || handle(stream, &queue));
    }

    Ok(())
}
