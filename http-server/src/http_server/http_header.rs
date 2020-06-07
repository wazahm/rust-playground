use std::collections::HashMap;

pub struct HttpHeader {
    data: HashMap<String, String>,
}

pub fn new() -> HttpHeader {
    HttpHeader { data: HashMap::new() }
}

impl HttpHeader {
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