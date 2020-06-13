use std::collections::HashMap;

pub struct HttpHeader {
    data: HashMap<String, String>,
}

pub fn new() -> HttpHeader {
    HttpHeader { data: HashMap::new() }
}

impl HttpHeader {
    pub fn get(&self, header: &str) -> &str {
        match self.data.get(&header.to_lowercase()) {
            Some(val) => val,
            None => ""
        }
    }
    pub fn set(&mut self, header: &str, value: &str) {
        self.data.insert(header.to_lowercase(), value.to_string());
    }
    pub fn remove(&mut self, header: &str) {
        self.data.remove(&header.to_lowercase());
    }
    pub fn to_map(&self) -> &HashMap<String, String> {
        &self.data
    }
}