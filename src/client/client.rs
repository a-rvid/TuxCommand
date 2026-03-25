use std::os::unix::net::UnixStream;
use std::io::BufReader;
use std::io::{self, Write, BufRead};
use std::thread;
use std::env;

fn client(socket_path: &str) {
    let mut stream = UnixStream::connect(socket_path).map_err(|e| eprintln!("Connection failed: {}", e)).unwrap();
    println!("Connected to {socket_path}");
    let reader = BufReader::new(stream.try_clone().unwrap());

    thread::spawn(move || {
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("{}", line);
            }
        }
    });

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        writeln!(stream, "{}", line).unwrap();
    }
}

fn main() { 
    client(&env::args().nth(1).unwrap_or("/tmp/tuxmux.sock".to_string()));
}