use std::collections::HashMap;

pub struct HttpRequest {
    pub header: HashMap<String, String>,
    pub body: Vec<u8>
}