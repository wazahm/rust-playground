#![allow(dead_code)]

use std::collections::HashMap;
use std::net:: { TcpListener, TcpStream };
use std::thread;
use std::io;

type HttpRequestHandler = fn(HashMap<String, String>, Vec<u8>);

#[derive(PartialEq)]
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    NONE
}

struct HttpRequest {
    header: HashMap<String, String>,
    body: Vec<u8>
}

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
    fn add(&mut self, url: String, method: HttpMethod,
            cb: HttpRequestHandler) {
        self.endpoints.push(HttpEndpoint {
            url: url,
            method: method,
            callback: cb
        });
    }
    pub fn get(&mut self, url: &String, callback: fn(HashMap<String, String>, Vec<u8>)) {
        self.add(url.clone(), HttpMethod::GET, callback);
    }
    pub fn post(&mut self, url: &String, callback: fn(HashMap<String, String>, Vec<u8>)) {
        self.add(url.clone(), HttpMethod::POST, callback);
    }
    pub fn put(&mut self, url: &String, callback: fn(HashMap<String, String>, Vec<u8>)) {
        self.add(url.clone(), HttpMethod::PUT, callback);
    }
    pub fn delete(&mut self, url: &String, callback: fn(HashMap<String, String>, Vec<u8>)) {
        self.add(url.clone(), HttpMethod::DELETE, callback);
    }
    fn parse_request(stream: TcpStream) -> HttpRequest {
        HttpRequest { header: HashMap::new(), body: Vec::new() }
    }
    fn get_request_handler(&self, url: &String, method: HttpMethod) -> Option<HttpRequestHandler> {
        for endpoint in self.endpoints {
            if (method == endpoint.method) && (url == &endpoint.url) {
                return Some(endpoint.callback);
            }
        }
        None
    }
    pub fn run(&mut self, ip: &str, port: u16) -> io::Result<()> {
        let listener = TcpListener::bind((ip, port))?;
    
        /* TcpListener::incoming() does accept() & returns the Result<TcpStream> */
        for conn in listener.incoming() {
            let mut stream = conn?;
            thread::spawn(move || {
                let mut request = HttpServer::parse_request(stream);
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
                match self.get_request_handler(url, method) {
                    Some(cb) => {
                        cb(request.header, request.body);
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