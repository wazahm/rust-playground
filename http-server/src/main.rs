use std::io::Read;
use std::net::{TcpStream, TcpListener};
use std::thread;

const SERVER_IP: &str = "127.0.0.1";
const SERVER_PORT: u16 = 8080;

fn handle_conn(stream: TcpStream) {

    let mut header = Vec::new();

    // TODO: Check when this stream.bytes() will stop
    for byte in stream.bytes() {
        match byte {
            Ok(x) => {
                header.push(x);
                // break when you receive end of HTTP header -> "/r/n/r/n"
            },
            Err(error) => {
                eprintln!("Error: {:?}", error);
                std::process::exit(0);
            }
        }
    }

    // Parse the header and get the "Content-Length" header value

    // Create a new Vector for http-body

    // Read from TcpStream and copy the no. of bytes mentioned in the "Content-Length" header

    // Process the header and the body

    // Send response
}

fn main() {
    let listener = match TcpListener::bind((SERVER_IP, SERVER_PORT)) {
        Ok(x) => x,
        Err(error) => {
            eprintln!("Error: {:?}", error);
            std::process::exit(0)
        },
    };

    /* TcpListener::incoming() does accept() & returns the Result<TcpStream> */
    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_conn(stream);
                });
            },
            Err(error) => {
                eprintln!("Error: {:?}", error);
            },
        };
    }
}
