#![allow(unused, dead_code)]

use std::collections::HashMap;
use std::net:: { TcpListener, TcpStream, Shutdown };
use std::io;
use std::thread;
use std::error::Error;
use std::sync::Arc;
use std::ops:: { Add, Deref };
use serde::{ Serialize, Deserialize };
use serde_json::Result as SerdeResult;
use std::io::{ Read, Write };
use std::convert::TryFrom;

const HTTP_VERSION: &str = "HTTP/1.1";
const CRLF: &str = "\r\n";
const DOUBLE_CRLF: &str = "\r\n\r\n";
const DOUBLE_CRLF_ASCII: [u8; 4] = ['\r' as u8, '\n' as u8, '\r' as u8, '\n' as u8];

const IANA_HTTP_RESPONSE_STATUS: [(u16, &str); 12] = [
    (200, "OK"),
    (301, "Moved Permanently"),
    (302, "Found"),
    (304, "Not Modified"),
    (400, "Bad Request"),
    (401, "Unauthorized"),
    (403, "Forbidden"),
    (404, "NotFound"),
    (500, "Internal Server Error"),
    (502, "Bad Gateway"),
    (503, "Service Not Available"),
    (505, "HTTP Version Not Supported")
    // TODO: Add all the IANA registered HTTP Response Codes/Reasons
];

const DEFAULT_RESPONSE_STATUS_CODE: u16 = IANA_HTTP_RESPONSE_STATUS[0].0;
const DEFAULT_RESPONSE_STATUS_REASON: &str = IANA_HTTP_RESPONSE_STATUS[0].1;

type HttpRequestHandler = fn(HttpRequest, HttpResponse);

#[derive(PartialEq, Clone)]
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    NONE
}

pub struct HttpRequest {
    pub header: HashMap<String, String>,
    pub body: Vec<u8>
}

struct HttpResponseStatus {
    code: u16,
    reason: String
}

pub struct HttpResponse {
    status: HttpResponseStatus,
    header: HashMap<&'static str, String>,
    header_sent: bool,
    has_body: bool,
    body: Vec<u8>,
    socket: TcpStream
}

impl HttpResponse {
    pub fn new(socket: TcpStream) -> HttpResponse {
        let status = HttpResponseStatus {
            code: DEFAULT_RESPONSE_STATUS_CODE,
            reason: DEFAULT_RESPONSE_STATUS_REASON.to_string()
        };

        let mut header: HashMap<&'static str, String> = HashMap::new();
        header.insert("Transfer-Encoding", String::from("chunked"));
        header.insert("Connection", String::from("close"));

        HttpResponse {
            status,
            header,
            header_sent: false,
            has_body: false,
            body: Vec::new(),
            socket
        }
    }
    pub fn status(&mut self, status: u16) -> &mut Self {
        self.status.code = status;

        // If the given code is not a IANA registered status code,
        // then by default the reason will be set to "unknown".
        self.status.reason = "unknown".to_string();

        for (code, reason) in &IANA_HTTP_RESPONSE_STATUS {
            if *code == status {
                self.status.reason = reason.to_string();
            }
        }

        self
    }
    pub fn status_message(&mut self, message: &str) -> &mut Self {
        self.status.reason = message.to_string();
        self
    }
    fn send_header(&mut self) -> Result<(), io::Error> {
        let sock = Write::by_ref(&mut self.socket);
        let mut line = String::from(HTTP_VERSION).add(" ")
                        .add(&self.status.code.to_string()).add(" ")
                        .add(&self.status.reason).add(CRLF);
        sock.write(line.as_bytes())?;

        for (key, value) in &self.header {
            line = String::from(*key).add(": ").add(value).add(CRLF);
            sock.write(line.as_bytes())?;
        }
        sock.write(CRLF.as_bytes())?;

        self.header_sent = true;

        Ok(())
    }
    fn get_chunk_size(chunk: &str) -> String {
        return format!("{:X}", chunk.as_bytes().len());
    }
    pub fn end(&mut self) -> Result<(), io::Error> {
        if !self.header_sent {
            self.send_header();
        }

        let sock = Write::by_ref(&mut self.socket);

        if self.has_body {
            sock.write(Self::get_chunk_size("").as_bytes())?;
            sock.write(DOUBLE_CRLF.as_bytes())?;
        }

        sock.flush()?;
        sock.shutdown(Shutdown::Both)?;
        Ok(())
    }
    pub fn write(&mut self, msg: &str) -> Result<(), io::Error> {
        if !self.header_sent {
            self.send_header();
        }

        let sock = Write::by_ref(&mut self.socket);

        sock.write(Self::get_chunk_size(msg).as_bytes())?;
        sock.write(CRLF.as_bytes())?;

        sock.write(msg.as_bytes())?;
        sock.write(CRLF.as_bytes())?;

        if !self.has_body {
            self.has_body = true;
        }

        Ok(())
    }
}

#[derive(Clone)]
struct HttpEndpoint {
    url: String,
    method: HttpMethod,
    callback: HttpRequestHandler
}

pub struct HttpServer {
    endpoints: Vec<HttpEndpoint>
}

impl HttpServer {
    pub fn new() -> HttpServer {
        HttpServer { endpoints: Vec::new() }
    }
    fn add(&mut self, url: &str, method: HttpMethod, cb: HttpRequestHandler) {
        self.endpoints.push(HttpEndpoint {
            url: String::from(url),
            method: method,
            callback: cb
        });
    }
    pub fn get(&mut self, url: &str, callback: HttpRequestHandler) {
        self.add(url, HttpMethod::GET, callback);
    }
    pub fn post(&mut self, url: &str, callback: HttpRequestHandler) {
        self.add(url, HttpMethod::POST, callback);
    }
    pub fn put(&mut self, url: &str, callback: HttpRequestHandler) {
        self.add(url, HttpMethod::PUT, callback);
    }
    pub fn delete(&mut self, url: &str, callback: HttpRequestHandler) {
        self.add(url, HttpMethod::DELETE, callback);
    }
    fn parse_request(stream: &mut TcpStream) -> Result<HttpRequest, Box<dyn Error>> {
        let mut header_buffer: Vec<u8> = Vec::new();
        let mut header_read = false;
        let stream_ref = Read::by_ref(stream);
        //let stream_ref = Read::by_ref(&mut stream);
        for _byte in stream_ref.bytes() {
            let byte = _byte?;
            header_buffer.push(byte);
            if header_buffer.ends_with(&DOUBLE_CRLF_ASCII) == true {
                header_read = true;
                break;
            }
        }

        if header_read == false {
            // stream ended; connection closed. But not received the complete HTTP header
            let custom_err = io::Error::new(io::ErrorKind::UnexpectedEof, "Incomplete HTTP header");
            return Result::Err(Box::new(custom_err));
        }

        let header = String::from_utf8(header_buffer)?;

        let mut header_map: HashMap<String, String> = HashMap::new();

        for (i, line) in header.split(CRLF).enumerate() {
            if i == 0 {
                // Parse the first line => GET /url HTTP/1.1
                let words: Vec<&str> = line.split(" ").collect();

                if words.len() != 3 {
                    let custom_err = io::Error::new(io::ErrorKind::InvalidData, "Invalid HTTP header");
                    return Result::Err(Box::new(custom_err));
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

        let mut body: Vec<u8> = Vec::new();
        if content_length > 0 {
            for _byte in stream_ref.bytes() {
                let byte = _byte?;
                body.push(byte);
                content_length -= 1;
                if content_length == 0 {
                    break;
                }
            }
        }

        Result::Ok(HttpRequest { header: header_map, body: body })
    }
    fn get_request_handler(endpoints: &Vec<HttpEndpoint>, url: &String, method: HttpMethod) -> Option<HttpRequestHandler> {
        for endpoint in endpoints {
            if (method == endpoint.method) && (url == &endpoint.url) {
                return Some(endpoint.callback);
            }
        }
        None
    }
    pub fn run(&self, ip: &str, port: u16) -> io::Result<()> {
        let endpoints = Arc::new(self.endpoints.clone());

        let listener = TcpListener::bind((ip, port))?;
        /* TcpListener::incoming() does accept() & returns the Result<TcpStream> */
        for conn in listener.incoming() {
            let mut stream = conn?;
            let endpoints_ref = endpoints.clone();
            thread::spawn(move || {
                let request: HttpRequest;
                match HttpServer::parse_request(&mut stream) {
                    Ok(req) => {
                        request = req;
                    },
                    Err(error) => {
                        eprintln!("Error: {:?}", error);
                        return;
                    }
                }

                let method: HttpMethod;
                match request.header.get("method") {
                    Some(x) => {
                        method = match x.as_str() {
                            "GET" => HttpMethod::GET,
                            "POST" => HttpMethod::POST,
                            "PUT" => HttpMethod::PUT,
                            "DELETE" => HttpMethod::DELETE,
                            _ => HttpMethod::NONE
                        }
                     },
                    None => {
                        // TODO: Send the right error code.
                        //       Close the connection properly.
                        return;
                    }
                }
                let url: &String;
                match request.header.get("url") {
                    Some(x) => { url = x },
                    None => {
                        // TODO: Send the right error code.
                        //       Close the connection properly.
                        return;
                    }
                }
                match HttpServer::get_request_handler(endpoints_ref.deref(), url, method) {
                    Some(cb) => {
                        cb(request, HttpResponse::new(stream));
                    }
                    None => {
                        // TODO: Send the right error code.
                        //       Close the connection properly.
                        return;
                    }
                }
            });
        }
        Ok(())
    }
}