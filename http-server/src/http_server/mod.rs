#![allow(unused, dead_code)]

use std::collections::HashMap;
use std::net::{ TcpListener, TcpStream };
use std::io;
use std::thread;
use std::error::Error;
use std::sync::Arc;
use std::ops::{ Add, Deref };
use std::io::Read;
use std::path::{ Path, PathBuf };
use log::info;

pub mod http_header;
pub mod http_request;
pub mod http_response;
mod to_bytes;

use http_header::*;
use http_request::HttpRequest;
use http_response::HttpResponse;

const CRLF: &str = "\r\n";
const DOUBLE_CRLF: &str = "\r\n\r\n";
const DOUBLE_CRLF_ASCII: [u8; 4] = ['\r' as u8, '\n' as u8, '\r' as u8, '\n' as u8];

type HttpRequestHandler = fn(&HttpRequest, &mut HttpResponse);
type HttpStaticHandler = fn(&Path, &mut HttpResponse);

#[derive(PartialEq, Clone, Copy)]
pub enum HttpVersion {
    V1_1,
    V2,
    UNKNOWN
}

impl HttpVersion {
    pub fn from_str(version: &str) -> Self {
        match version {
            "HTTP/1.1" => Self::V1_1,
            "HTTP/2" => Self::V2,
            _ => Self::UNKNOWN
        }
    }
    pub fn to_str(self) -> &'static str {
        match self {
            Self::V1_1 => "HTTP/1.1",
            Self::V1_1 => "HTTP/2",
            _ => ""
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    UNKNOWN
}

impl HttpMethod {
    pub fn from_str(method: &str) -> Self {
        match method {
            "GET" => Self::GET,
            "POST" => Self::POST,
            "PUT" => Self::PUT,
            "DELETE" => Self::DELETE,
            _ => Self::UNKNOWN
        }
    }
    pub fn to_str(self) -> &'static str {
        match self {
            Self::GET => "GET",
            Self::POST => "POST",
            Self::PUT => "PUT",
            Self::DELETE => "DELETE",
            _ => ""
        }
    }
}

#[derive(Clone)]
struct HttpEndpoint {
    url: String,
    method: HttpMethod,
    callback: HttpRequestHandler
}

#[derive(Clone)]
struct HttpStaticPath {
    prefix: String,
    path: PathBuf
}

pub struct HttpServer {
    endpoints: Vec<HttpEndpoint>,
    static_paths: Vec<HttpStaticPath>,
    static_handler: HttpStaticHandler
}

impl HttpServer {
    pub fn new() -> HttpServer {
        HttpServer {
            endpoints: Vec::new(),
            static_paths: Vec::new(),
            static_handler: | file, response | {
                response.send_file(file);
            }
        }
    }
    pub fn static_path(&mut self, prefix: &str, path: &Path) {
        let mut prefix = prefix.to_owned();
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        self.static_paths.push(HttpStaticPath {
            prefix: prefix,
            path: path.to_owned()
        });
    }
    pub fn static_serve(&mut self, cb: HttpStaticHandler) {
        self.static_handler = cb;
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
    fn parse_request(socket: &mut TcpStream) -> Result<Option<HttpRequest>, Box<dyn Error>> {
        let mut header_buf: Vec<u8> = Vec::new();
        let mut header_read = false;
        let socket = Read::by_ref(socket);
        for _byte in socket.bytes() {
            let byte = _byte?;
            header_buf.push(byte);
            if header_buf.ends_with(&DOUBLE_CRLF_ASCII) == true {
                header_read = true;
                break;
            }
        }

        /* No data read, connection closed by the peer */
        /* So we can silently ignore the connection establishment */
        if header_buf.len() == 0 {
            return Ok(None)
        }

        if header_read == false {
            // socket ended; connection closed. But not received the complete HTTP header
            let custom_err = io::Error::new(io::ErrorKind::UnexpectedEof, "Incomplete HTTP header");
            return Result::Err(Box::new(custom_err));
        }

        let header_buf = String::from_utf8(header_buf)?;

        let mut http_version = HttpVersion::UNKNOWN;
        let mut http_method = HttpMethod::UNKNOWN;
        let mut req_url = String::new();
        let mut header = http_header::new();

        for (i, line) in header_buf.split(CRLF).enumerate() {
            if i == 0 {
                // Parse the first line => GET /url HTTP/1.1
                let words: Vec<&str> = line.split(" ").collect();

                if words.len() != 3 {
                    let custom_err = io::Error::new(io::ErrorKind::InvalidData, "Invalid HTTP header");
                    return Result::Err(Box::new(custom_err));
                }

                http_method = HttpMethod::from_str(words[0]);
                req_url = String::from(words[1]);
                http_version = HttpVersion::from_str(words[2]);
            } else {
                let field_value: Vec<&str> = line.split(":").map(|x| x.trim()).collect();
                if field_value.len() != 2 {
                    continue;
                } else {
                    // TODO: Deal with the HTTP fields which has multiple values or key-value pairs within the value part
                    header.set(field_value[0], field_value[1]);
                }
            }
        }

        let mut content_length = 0;
        let x = header.get("content-length");
        if !x.is_empty() {
            content_length = x.parse::<u32>()?;
        }

        let mut body: Vec<u8> = Vec::new();
        if content_length > 0 {
            for _byte in socket.bytes() {
                let byte = _byte?;
                body.push(byte);
                content_length -= 1;
                if content_length == 0 {
                    break;
                }
            }
        }

        Ok(Some(HttpRequest {
            http_version,
            method: http_method,
            url: req_url,
            header,
            body
        }))
    }
    fn get_request_handler(endpoints: &Vec<HttpEndpoint>, url: &String, method: HttpMethod) -> Option<HttpRequestHandler> {
        for endpoint in endpoints {
            if (method == endpoint.method) && (url == &endpoint.url) {
                return Some(endpoint.callback);
            }
        }
        None
    }
    fn get_static_file(static_paths: &Vec<HttpStaticPath>, url: &String) -> Option<PathBuf> {
        for sp in static_paths {
            if url.starts_with(&sp.prefix) {
                let mut file_path = PathBuf::from(&sp.path);
                file_path.push(&url[sp.prefix.len()..]);
                if file_path.exists() {
                    return Some(file_path)
                }
            }
        }
        None
    }
    fn worker_job(mut socket: TcpStream,
                  endpoints: &Vec<HttpEndpoint>,
                  static_paths: &Vec<HttpStaticPath>,
                  static_handler: &HttpStaticHandler) ->  Result<Option<TcpStream>, Box<dyn Error>> {

        let opt_request = HttpServer::parse_request(&mut socket)?;

        let request = match opt_request {
            Some(x) => x,
            None => return Ok(None)
        };

        info!("Client - {:?} | Request - {} {}", socket.peer_addr().unwrap(), request.method.to_str(), &request.url);

        if let Some(cb)= Self::get_request_handler(endpoints, &request.url, request.method) {
            cb(&request, &mut http_response::new(&mut socket, &request));
        }
        else if let Some(file_path) = Self::get_static_file(static_paths, &request.url) {
            static_handler(&file_path, &mut http_response::new(&mut socket, &request));
        }
        else {
            http_response::new(&mut socket, &request).status(404).end();
        }

        // If the connection is keep-alive,
        // return the TcpStream, so that it can be used for next request.
        let conn_value = request.header.get("connection");
        if(conn_value.to_lowercase() == "keep-alive") {
            Ok(Some(socket))
        }
        else {
            Ok(None)
        }
    }
    pub fn run(&self, ip: &str, port: u16) -> io::Result<()> {
        let endpoints = Arc::new(self.endpoints.clone());
        let static_paths = Arc::new(self.static_paths.clone());
        let static_handler = Arc::new(self.static_handler);

        let listener = TcpListener::bind((ip, port))?;
        /* TcpListener::incoming() does accept() & returns the Result<TcpStream> */
        for conn in listener.incoming() {
            let mut socket = conn?;

            let endpoints = endpoints.clone();
            let static_paths = static_paths.clone();
            let static_handler = static_handler.clone();

            thread::spawn(move || {
                info!("Connected to the client - {:?}", socket.peer_addr().unwrap());
                loop {
                    let result = Self::worker_job(socket, &endpoints, &static_paths, &static_handler);
                    match result {
                        Ok(opt_socket) => {
                            match opt_socket {
                                Some(s) => { socket = s; },
                                None => { break; }
                            }
                        },
                        Err(error) => {
                            eprintln!("Error: {:?}", error);
                            break;
                        }
                    }
                }
            });
        }
        Ok(())
    }
}