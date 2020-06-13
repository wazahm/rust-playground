use std::net::{ TcpStream, Shutdown };
use std::io;
use std::io::{ Read, Write };
use std::path::Path;
use std::fs::File;
use std::ops::Add;
use serde::Serialize;
use mime_guess;

use super::*;
use super::http_request::HttpRequest;
use super::http_header::HttpHeader;
use super::to_bytes::ToBytes;

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

struct HttpResponseStatus {
    code: u16,
    reason: String
}

pub struct HttpResponse<'a> {
    status: HttpResponseStatus,
    pub header: HttpHeader,
    header_sent: bool,
    chunked_body: bool,
    body: Vec<u8>,
    socket: &'a mut TcpStream
}

pub fn new<'a>(socket: &'a mut TcpStream, request: &HttpRequest) -> HttpResponse<'a> {
    let status = HttpResponseStatus {
        code: DEFAULT_RESPONSE_STATUS_CODE,
        reason: DEFAULT_RESPONSE_STATUS_REASON.to_string()
    };

    let mut header = http_header::new();
    HttpResponse::add_default_headers(&request.header, &mut header);

    HttpResponse {
        status,
        header,
        header_sent: false,
        chunked_body: false,
        body: Vec::new(),
        socket
    }
}

impl<'a> HttpResponse<'a> {
    fn add_default_headers(req_header: &HttpHeader, res_header: &mut HttpHeader) {
        if req_header.get("connection").to_lowercase() == "keep-alive" {
            res_header.set("Connection", "keep-alive");
        }
        else {
            res_header.set("Connection", "close");
        }
    }
    fn send_header(&mut self) -> Result<(), io::Error> {
        let sock = Write::by_ref(self.socket);
        let http_version = HttpVersion::V1_1.to_str();
        let mut line = String::from(http_version).add(" ")
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

        let sock = Write::by_ref(self.socket);

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
    fn _end(&mut self) -> Result<(), io::Error> {
        let sock = Write::by_ref(self.socket);
        sock.flush()?;
        if self.header.get("connection") == "close" {
            sock.shutdown(Shutdown::Both)?;
        }
        Ok(())
    }
    pub fn end(&mut self) -> Result<(), io::Error> {
        if !self.header_sent {
            self.send_header()?;
        }

        let sock = Write::by_ref(self.socket);

        if self.chunked_body {
            sock.write(&['0' as u8])?;
            sock.write(DOUBLE_CRLF.as_bytes())?;
        }
        self._end()
    }
    pub fn send(&mut self, data: impl ToBytes) -> Result<(), io::Error> {
        let data = data.to_bytes();

        if !self.header_sent {
            self.header.set("content-length", &data.len().to_string());
            self.send_header()?;
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "HTTP header is already sent. Cannot send it again."));
        }

        let sock = Write::by_ref(self.socket);
        sock.write(data)?;

        self._end()
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
    fn get_file_name(path: &Path) -> Result<&str, io::Error> {
        if !path.metadata()?.is_file() {
            return Err(io::Error::new(io::ErrorKind::Other, "Not a file"))
        }
        Ok(path.file_name().unwrap().to_str().unwrap())
    }
    pub fn send_file(&mut self, path: &Path) -> Result<(), io::Error> {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        self.content_type(mime.essence_str());

        let mut data = Vec::new();
        File::open(path)?.read_to_end(&mut data)?;
        self.send(&data[..])
    }
    pub fn download(&mut self, path: &Path) -> Result<(), io::Error> {
        let file_name = Self::get_file_name(path)?;
        self.header.set("Content-Disposition", &(format!("attachment; filename={}", file_name)));
        self.send_file(path)
    }
}