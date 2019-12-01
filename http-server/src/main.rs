use std::io::Write;
use std::net::{TcpStream, TcpListener};
use std::thread;
use std::collections::HashMap;
use std::io::Read;

const SERVER_IP: &str = "127.0.0.1";
const SERVER_PORT: u16 = 8080;

const CRLF_STR: &str = "\r\n";
const DOUBLE_CRLF_ASCII: [u8; 4] = ['\r' as u8, '\n' as u8, '\r' as u8, '\n' as u8];

fn handle_conn(stream: TcpStream) {

    let mut header_buffer = Vec::new();
    let mut header_read = false;
    let mut stream_ref = stream.by_ref();
    for byte in stream_ref.bytes() {
        match byte {
            Ok(x) => {
                header_buffer.push(x);
                if header_buffer.ends_with(&DOUBLE_CRLF_ASCII) == true {
                    header_read = true;
                    break;
                }
            },
            Err(error) => {
                eprintln!("Error: {:?}", error);
                return;
            }
        }
    }

    if header_read == false {
        // stream ended. But not received the complete HTTP header
        return;
    }

    let header: String;

    match String::from_utf8(header_buffer) {
        Ok(x) => {
            header = x
        },
        Err(error) => {
            eprintln!("Error: {:?}", error);
            return;
        }
    }

    let mut header_map: HashMap<String, String> = HashMap::new();

    for (i, line) in header.split(CRLF_STR).enumerate() {
        if i == 0 {
            // Parse the first line => GET /url HTTP/1.1
            let words: Vec<&str> = line.split(" ").collect();
    
            if words.len() != 3 {
                return;
            }
    
            // TODO: Validate before inserting items in the hash map
            header_map.insert(String::from("method"), String::from(words[0]));
            header_map.insert(String::from("url"), String::from(words[1]));
            header_map.insert(String::from("http_version"), String::from(words[2].trim_start_matches("HTTP/")));
        } else {
            let field_value: Vec<&str> = line.trim().split(":").collect();
            if field_value.len() != 2 {
                continue;
            } else {
                // TODO: Deal with the HTTP fields which has multiple values or key-value pairs within the value part
                let header_field = String::from(field_value[0]).to_lowercase();
                header_map.insert(header_field, String::from(field_value[1]));
            }
        }
    }

    let mut body_buffer = Vec::new();

    match header_map.get("content-length") {
        Some(x) => {
            let mut content_length;
            match x.parse::<u32>() {
                Ok(x) => {
                    content_length = x;
                    if content_length > 0 {
                        for byte in stream_ref.bytes() {
                            body_buffer.push(byte);
                            content_length -= 1;
                            if content_length == 0 {
                                break;
                            }
                        }
                    }
                },
                Err(error) => {
                    eprintln!("Error: {:?}", error);
                }
            }
        },
        None => {}
    }

    // Create a new Vector for http-body

    // Read from TcpStream and copy the no. of bytes mentioned in the "Content-Length" header

    // Process the header and the body

    // Send response
    let response = "200 OK\r\nContent-Type: 2\r\n\r\nHi";
    stream_ref.write(response.as_bytes());
}

fn main() {

    let buffer = vec!['a' as u8, 'b' as u8, 'C' as u8, 'D' as u8, '*' as u8, '%' as u8,];

    println!("{}", String::from_utf8(buffer).unwrap());

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
