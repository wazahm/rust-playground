#![allow(dead_code)]

use std::collections::HashMap;
use std::net:: { TcpListener, TcpStream };
use std::thread;
use std::io;
use std::sync::Arc;
use std::ops::Deref;

type HttpRequestHandler = fn(HashMap<String, String>, Vec<u8>);

#[derive(PartialEq, Clone)]
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    NONE
}

struct HttpRequest {
    socket: TcpStream,
    header: HashMap<String, String>,
    body: Vec<u8>
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
    fn add(&mut self, url: String, method: HttpMethod, cb: HttpRequestHandler) {
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
        HttpRequest { socket: stream, header: HashMap::new(), body: Vec::new() }
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
            let stream = conn?;
            let endpoints_ref = endpoints.clone();
            thread::spawn(move || {
                let request = HttpServer::parse_request(stream);
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