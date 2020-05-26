#![allow(unused, dead_code)]

use std::collections::HashMap;
use std::net:: { TcpListener, TcpStream, Shutdown };
use std::io;
use std::thread;
use std::path::Path;
use std::fs::File;
use std::error::Error;
use std::sync::Arc;
use std::ops:: { Add, Deref };
use serde::{ Serialize, Deserialize };
use serde_json::Result as SerdeResult;
use std::io::{ Read, Write };
use mime_guess;

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

pub trait ToBytes {
    fn to_bytes(&self) -> &[u8];
}

impl ToBytes for &[u8] {
    fn to_bytes(&self) -> &[u8] {
        self
    }
}

impl ToBytes for &str {
    fn to_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl ToBytes for String {
    fn to_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

impl ToBytes for &String {
    fn to_bytes(&self) -> &[u8] {
        self.as_bytes()
    }
}

type HttpRequestHandler = fn(HttpRequest, HttpResponse);

#[derive(PartialEq, Clone)]
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    NONE
}

pub struct HttpHeader {
    data: HashMap<String, String>,
}
impl HttpHeader {
    pub fn new() -> HttpHeader {
        HttpHeader { data: HashMap::new() }
    }
    pub fn get(&self, header: &str) -> Option<&String> {
        self.data.get(header)
    }
    pub fn set(&mut self, header: &str, value: &str) {
        self.data.insert(header.to_string(), value.to_string());
    }
    pub fn remove(&mut self, header: &str) {
        self.data.remove(header);
    }
    pub fn to_map(&self) -> &HashMap<String, String> {
        &self.data
    }
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
    pub header: HttpHeader,
    header_sent: bool,
    chunked_body: bool,
    body: Vec<u8>,
    socket: TcpStream
}

impl HttpResponse {
    fn new(socket: TcpStream) -> HttpResponse {
        let status = HttpResponseStatus {
            code: DEFAULT_RESPONSE_STATUS_CODE,
            reason: DEFAULT_RESPONSE_STATUS_REASON.to_string()
        };

        let mut header = HttpHeader::new();
        Self::add_default_headers(&mut header);

        HttpResponse {
            status,
            header,
            header_sent: false,
            chunked_body: false,
            body: Vec::new(),
            socket
        }
    }
    fn add_default_headers(header: &mut HttpHeader) {
        header.set("Connection", "close");
    }
    fn send_header(&mut self) -> Result<(), io::Error> {
        let sock = Write::by_ref(&mut self.socket);
        let mut line = String::from(HTTP_VERSION).add(" ")
                        .add(&self.status.code.to_string()).add(" ")
                        .add(&self.status.reason).add(CRLF);
        sock.write(line.as_bytes())?;

        for (key, value) in self.header.to_map() {
            if !key.is_empty() && !value.is_empty() {
                line = key.to_string().add(": ").add(value).add(CRLF);
                sock.write(line.as_bytes())?;
            }
        }
        sock.write(CRLF.as_bytes())?;

        self.header_sent = true;

        Ok(())
    }
    fn sock_close(&mut self) -> Result<(), io::Error> {
        let sock = Write::by_ref(&mut self.socket);
        sock.flush()?;
        sock.shutdown(Shutdown::Both)
    }
    fn get_chunk_size(chunk: &[u8]) -> String {
        return format!("{:X}", chunk.len());
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
    pub fn write(&mut self, data: impl ToBytes) -> Result<(), io::Error> {
        if !self.header_sent {
            self.header.set("Transfer-Encoding", "chunked");
            self.send_header()?;
        }

        let sock = Write::by_ref(&mut self.socket);

        let data = data.to_bytes();
        sock.write(Self::get_chunk_size(data).as_bytes())?;
        sock.write(CRLF.as_bytes())?;

        sock.write(data)?;
        sock.write(CRLF.as_bytes())?;

        if !self.chunked_body {
            self.chunked_body = true;
        }

        Ok(())
    }
    pub fn end(&mut self) -> Result<(), io::Error> {
        if !self.header_sent {
            self.send_header()?;
        }

        let sock = Write::by_ref(&mut self.socket);

        if self.chunked_body {
            sock.write(&['0' as u8])?;
            sock.write(DOUBLE_CRLF.as_bytes())?;
        }
        self.sock_close()
    }
    pub fn send(&mut self, data: impl ToBytes) -> Result<(), io::Error> {
        let data = data.to_bytes();

        if !self.header_sent {
            self.header.set("Content-Length", &data.len().to_string());
            self.send_header()?;
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "HTTP header is already sent. Cannot send it again."));
        }

        let sock = Write::by_ref(&mut self.socket);
        sock.write(data)?;

        self.sock_close()
    }
    pub fn json(&mut self, value: &impl Serialize) -> Result<(), io::Error> {
        self.content_type("application/json");
        let data = serde_json::to_string(value)?;
        self.send(data)
    }
    pub fn content_type(&mut self, value: &str) -> &mut Self {
        self.header.set("Content-Type", value);
        self
    }
    pub fn redirect(&mut self, location: &str) -> Result<(), io::Error> {
        self.status(302);
        self.header.set("Location", location);
        self.end()
    }
    fn get_file_name(path: &str) -> Result<&str, io::Error> {
        /* Will this work on Windows? */
        let path = Path::new(path);
        if !path.metadata()?.is_file() {
            return Err(io::Error::new(io::ErrorKind::Other, "Not a file"))
        }
        Ok(path.file_name().unwrap().to_str().unwrap())
    }
    pub fn send_file(&mut self, path: &str) -> Result<(), io::Error> {
        let path = Path::new(path);
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        self.content_type(mime.essence_str());

        let mut data = Vec::new();
        File::open(path)?.read_to_end(&mut data)?;
        self.send(&data[..])
    }
    pub fn download(&mut self, path: &str) -> Result<(), io::Error> {
        let file_name = Self::get_file_name(path)?;
        self.header.set("Content-Disposition", &(format!("attachment; filename={}", file_name)));
        self.send_file(path)
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