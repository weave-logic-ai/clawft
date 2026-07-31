//! Minimal hex helpers (avoid pulling an extra workspace dep for 40 lines).

use std::fmt;

#[derive(Debug)]
pub struct HexError(String);

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

pub fn hex_to_bytes(s: &str) -> Result<Vec<u8>, HexError> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return Err(HexError("odd length hex".into()));
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = from_nibble(bytes[i])?;
        let lo = from_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
        i += 2;
    }
    Ok(out)
}

fn from_nibble(c: u8) -> Result<u8, HexError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(HexError(format!("invalid hex char: {}", c as char))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let v = vec![0u8, 15, 16, 255];
        let h = bytes_to_hex(&v);
        assert_eq!(h, "000f10ff");
        assert_eq!(hex_to_bytes(&h).unwrap(), v);
    }
}
