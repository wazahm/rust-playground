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

pub mod http_header;
pub mod http_request;
pub mod http_response;
mod to_bytes;

use http_request::HttpRequest;
use http_response::HttpResponse;

const HTTP_VERSION: &str = "HTTP/1.1";
const CRLF: &str = "\r\n";
const DOUBLE_CRLF: &str = "\r\n\r\n";
const DOUBLE_CRLF_ASCII: [u8; 4] = ['\r' as u8, '\n' as u8, '\r' as u8, '\n' as u8];

type HttpRequestHandler = fn(HttpRequest, HttpResponse);
type HttpStaticHandler = fn(&Path, HttpResponse);

#[derive(PartialEq, Clone)]
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    NONE
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
            static_handler: | file, mut response | {
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
    pub fn run(&self, ip: &str, port: u16) -> io::Result<()> {
        let endpoints = Arc::new(self.endpoints.clone());
        let static_paths = Arc::new(self.static_paths.clone());
        let static_handler = Arc::new(self.static_handler);

        let listener = TcpListener::bind((ip, port))?;
        /* TcpListener::incoming() does accept() & returns the Result<TcpStream> */
        for conn in listener.incoming() {
            let mut stream = conn?;

            let endpoints = endpoints.clone();
            let static_paths = static_paths.clone();
            let static_handler = static_handler.clone();

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
                if let Some(cb)= HttpServer::get_request_handler(&endpoints, url, method) {
                    cb(request, http_response::new(stream));
                }
                else if let Some(file_path) = HttpServer::get_static_file(&static_paths, url) {
                    (static_handler)(&file_path, http_response::new(stream));
                }
                else {
                    http_response::new(stream).status(404).end();
                }
            });
        }
        Ok(())
    }
}