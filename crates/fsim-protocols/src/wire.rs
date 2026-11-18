//! Protobuf-3 binary wire codec (varint, 64-bit, length-delimited, 32-bit).
//!
//! Ensures forward and backward wire compatibility:
//! - Unknown field numbers are skipped cleanly without error (older readers can decode newer messages).
//! - Missing fields default to proto3 zero values (newer readers can decode older messages).
//! - Decoders never panic on malformed bytes (`Result<T, WireError>`).

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum WireError {
    #[error("unexpected end of buffer")]
    UnexpectedEof,
    #[error("varint overflow")]
    VarintOverflow,
    #[error("invalid wire type {0}")]
    InvalidWireType(u8),
    #[error("invalid utf-8 string")]
    InvalidUtf8,
    #[error("length exceeds remaining bytes")]
    LengthOutOfBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WireType {
    Varint = 0,
    Fixed64 = 1,
    LengthDelimited = 2,
    Fixed32 = 5,
}

impl WireType {
    pub fn from_u8(v: u8) -> Result<Self, WireError> {
        match v {
            0 => Ok(Self::Varint),
            1 => Ok(Self::Fixed64),
            2 => Ok(Self::LengthDelimited),
            5 => Ok(Self::Fixed32),
            other => Err(WireError::InvalidWireType(other)),
        }
    }
}

pub fn encode_varint(mut value: u64, buf: &mut Vec<u8>) {
    while value >= 0x80 {
        buf.push(((value as u8) & 0x7F) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
}

pub fn decode_varint(buf: &[u8], pos: &mut usize) -> Result<u64, WireError> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        if *pos >= buf.len() {
            return Err(WireError::UnexpectedEof);
        }
        let byte = buf[*pos];
        *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if (byte & 0x80) == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err(WireError::VarintOverflow);
        }
    }
}

pub fn encode_tag(field_number: u32, wire_type: WireType, buf: &mut Vec<u8>) {
    let tag = ((field_number as u64) << 3) | (wire_type as u64);
    encode_varint(tag, buf);
}

pub fn decode_tag(buf: &[u8], pos: &mut usize) -> Result<(u32, WireType), WireError> {
    let tag = decode_varint(buf, pos)?;
    let field_number = (tag >> 3) as u32;
    if field_number == 0 {
        return Err(WireError::InvalidWireType((tag & 0x07) as u8));
    }
    let wire_type = WireType::from_u8((tag & 0x07) as u8)?;
    Ok((field_number, wire_type))
}

pub fn encode_uint64_field(field_number: u32, value: u64, buf: &mut Vec<u8>) {
    if value != 0 {
        encode_tag(field_number, WireType::Varint, buf);
        encode_varint(value, buf);
    }
}

pub fn encode_int64_field(field_number: u32, value: i64, buf: &mut Vec<u8>) {
    if value != 0 {
        encode_tag(field_number, WireType::Varint, buf);
        encode_varint(value as u64, buf);
    }
}

pub fn encode_uint32_field(field_number: u32, value: u32, buf: &mut Vec<u8>) {
    if value != 0 {
        encode_tag(field_number, WireType::Varint, buf);
        encode_varint(value as u64, buf);
    }
}

pub fn encode_bool_field(field_number: u32, value: bool, buf: &mut Vec<u8>) {
    if value {
        encode_tag(field_number, WireType::Varint, buf);
        encode_varint(1, buf);
    }
}

pub fn encode_bytes_field(field_number: u32, value: &[u8], buf: &mut Vec<u8>) {
    if !value.is_empty() {
        encode_tag(field_number, WireType::LengthDelimited, buf);
        encode_varint(value.len() as u64, buf);
        buf.extend_from_slice(value);
    }
}

pub fn encode_string_field(field_number: u32, value: &str, buf: &mut Vec<u8>) {
    if !value.is_empty() {
        encode_bytes_field(field_number, value.as_bytes(), buf);
    }
}

pub fn decode_bytes<'a>(buf: &'a [u8], pos: &mut usize) -> Result<&'a [u8], WireError> {
    let len = decode_varint(buf, pos)? as usize;
    let end = pos.checked_add(len).ok_or(WireError::LengthOutOfBounds)?;
    if end > buf.len() {
        return Err(WireError::LengthOutOfBounds);
    }
    let slice = &buf[*pos..end];
    *pos = end;
    Ok(slice)
}

pub fn decode_string(buf: &[u8], pos: &mut usize) -> Result<String, WireError> {
    let bytes = decode_bytes(buf, pos)?;
    std::str::from_utf8(bytes)
        .map(|s| s.to_owned())
        .map_err(|_| WireError::InvalidUtf8)
}

pub fn skip_field(wire_type: WireType, buf: &[u8], pos: &mut usize) -> Result<(), WireError> {
    match wire_type {
        WireType::Varint => {
            let _ = decode_varint(buf, pos)?;
            Ok(())
        }
        WireType::Fixed64 => {
            let end = pos.checked_add(8).ok_or(WireError::UnexpectedEof)?;
            if end > buf.len() {
                return Err(WireError::UnexpectedEof);
            }
            *pos = end;
            Ok(())
        }
        WireType::LengthDelimited => {
            let _ = decode_bytes(buf, pos)?;
            Ok(())
        }
        WireType::Fixed32 => {
            let end = pos.checked_add(4).ok_or(WireError::UnexpectedEof)?;
            if end > buf.len() {
                return Err(WireError::UnexpectedEof);
            }
            *pos = end;
            Ok(())
        }
    }
}

pub trait ProtoMessage: Sized {
    fn encode_proto(&self, buf: &mut Vec<u8>);
    fn decode_proto(buf: &[u8]) -> Result<Self, WireError>;

    fn to_proto_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode_proto(&mut buf);
        buf
    }
}
