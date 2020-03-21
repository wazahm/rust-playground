use std::ops::Add;
use std::io::{Read, Write};
use std::net::{TcpStream, TcpListener};
use std::thread;
use std::collections::HashMap;

const SERVER_IP: &str = "127.0.0.1";
const SERVER_PORT: u16 = 8080;

const CRLF: &str = "\r\n";
const DOUBLE_CRLF_ASCII: [u8; 4] = ['\r' as u8, '\n' as u8, '\r' as u8, '\n' as u8];

/*
struct HttpRequest {
    header: HashMap<String, String>,
    body: Vec<u8>
}


fn parse_request(stream: &TcpStream) -> HttpRequest {

}
*/

fn handle_conn(mut stream: TcpStream) {
    let mut header_buffer: Vec<u8> = Vec::new();
    let mut header_read = false;
    let stream_ref = Read::by_ref(&mut stream);
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
            header = x;
        },
        Err(error) => {
            eprintln!("Error: {:?}", error);
            return;
        }
    }

    let mut header_map: HashMap<String, String> = HashMap::new();

    for (i, line) in header.split(CRLF).enumerate() {
        if i == 0 {
            // Parse the first line => GET /url HTTP/1.1
            let words: Vec<&str> = line.split(" ").collect();

            if words.len() != 3 {
                return;
            }

            // TODO: Validate before inserting items in the hash map
            header_map.insert(String::from("method"), String::from(words[0]));
            header_map.insert(String::from("url"), String::from(words[1]));
            header_map.insert(String::from("http-version"), String::from(words[2].trim_start_matches("HTTP/")));
        } else {
            let field_value: Vec<&str> = line.split(":").map(|x| x.trim()).collect();
            if field_value.len() != 2 {
                continue;
            } else {
                // TODO: Deal with the HTTP fields which has multiple values or key-value pairs within the value part
                header_map.insert(field_value[0].to_lowercase(), String::from(field_value[1]));
            }
        }
    }

    let mut content_length = 0;
    match header_map.get("content-length") {
        Some(x) => {
            match x.parse::<u32>() {
                Ok(x) => {
                    content_length = x;
                },
                Err(error) => {
                    eprintln!("Error: {:?}", error);
                }
            }
        },
        None => {}
    }

    let mut body_buffer: Vec<u8> = Vec::new();
    if content_length > 0 {
        for byte in stream_ref.bytes() {
            match byte {
                Ok(x) => {
                    body_buffer.push(x);
                    content_length -= 1;
                    if content_length == 0 {
                        break;
                    }
                },
                Err(error) => {
                    eprintln!("Error: {:?}", error);
                    return;
                }
            }
        }
    }

    let mut body = String::new();
    match String::from_utf8(body_buffer) {
        Ok(x) => {
            body += &x;
        },
        Err(error) => {
            eprintln!("Error: {:?}", error);
        }
    }

    println!("\n--------------HTTP Request---------");
    println!("{:?}\n", header_map);
    println!("{}", body);
    println!("--------------END---------\n");

    // TODO: Change these based on the endpoint
    let response_body = "Hi";
    let response_length = response_body.len().to_string();
    let content_type = "text/plain";

    let mut response = String::new();
    response = response.add("HTTP/1.1 200 OK").add(CRLF);
    response = response.add("Connection: close").add(CRLF);
    response = response.add("Content-Type: ").add(&content_type).add(CRLF);
    response = response.add("Content-Length: ").add(&response_length).add(CRLF).add(CRLF);
    response = response.add(response_body);

    println!("\n--------------HTTP Response---------");
    println!("{}", response);
    println!("--------------END---------\n");

    let response_bytes: Vec<u8> = response.chars().map(|x| x as u8).collect();
    match stream.write(&response_bytes) {
        Err(error) => println!("Error: {:?}", error),
        _ => {}
    }
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
