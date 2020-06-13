use std::collections::HashMap;
use super::*;
use super::http_header::HttpHeader;

pub struct HttpRequest {
    pub http_version: HttpVersion,
    pub method: HttpMethod,
    pub url: String,
    pub header: HttpHeader,
    pub body: Vec<u8>
}