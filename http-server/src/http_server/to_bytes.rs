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