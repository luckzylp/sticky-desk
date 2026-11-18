use crate::app::tray::TrayCommand;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::Sender;
use std::time::Duration;

const ADDR: &str = "127.0.0.1:39017";

/// Returns true when this process owns the instance lock and the listener started.
/// Returns false when another instance is already running (the caller should exit).
pub fn start(sender: Sender<TrayCommand>) -> bool {
    let listener = match TcpListener::bind(ADDR) {
        Ok(listener) => listener,
        Err(_) => {
            // Another instance owns the port. Ask it for a new note.
            if let Ok(mut stream) = TcpStream::connect(ADDR) {
                let _ = stream.write_all(b"new-note\n");
                let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
            }
            return false;
        }
    };
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut buf = [0u8; 32];
            let n = stream.read(&mut buf).unwrap_or(0);
            let message = String::from_utf8_lossy(&buf[..n]);
            let cmd = if message.contains("show") {
                TrayCommand::ShowAllNotes
            } else if message.contains("explorer") {
                TrayCommand::Explorer
            } else {
                TrayCommand::NewNote
            };
            let _ = sender.send(cmd);
        }
    });
    true
}
