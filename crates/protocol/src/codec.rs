//! Low-level encoding: VarInts, strings and length-prefixed packet frames.

use anyhow::{bail, Context, Result};
use std::io::{Read, Write};

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

/// Maximum size of a single packet frame (2^21 - 1, the largest 3-byte VarInt).
pub const MAX_PACKET_LEN: usize = 2_097_151;

pub fn write_varint(buf: &mut Vec<u8>, value: i32) {
    let mut v = value as u32;
    loop {
        if v & !(SEGMENT_BITS as u32) == 0 {
            buf.push(v as u8);
            return;
        }
        buf.push((v as u8 & SEGMENT_BITS) | CONTINUE_BIT);
        v >>= 7;
    }
}

pub fn varint_len(value: i32) -> usize {
    let mut tmp = Vec::with_capacity(5);
    write_varint(&mut tmp, value);
    tmp.len()
}

pub fn read_varint<R: Read>(r: &mut R) -> Result<i32> {
    let mut value: u32 = 0;
    for i in 0..5 {
        let mut byte = [0u8];
        r.read_exact(&mut byte).context("reading VarInt")?;
        value |= ((byte[0] & SEGMENT_BITS) as u32) << (7 * i);
        if byte[0] & CONTINUE_BIT == 0 {
            return Ok(value as i32);
        }
    }
    bail!("VarInt is too big")
}

pub fn write_string(buf: &mut Vec<u8>, s: &str) {
    write_varint(buf, s.len() as i32);
    buf.extend_from_slice(s.as_bytes());
}

pub fn read_string<R: Read>(r: &mut R, max_chars: usize) -> Result<String> {
    let len = read_varint(r)?;
    if len < 0 || len as usize > max_chars * 3 {
        bail!("string length {len} out of range");
    }
    let mut bytes = vec![0u8; len as usize];
    r.read_exact(&mut bytes)?;
    Ok(String::from_utf8(bytes)?)
}

/// Builds the body of a packet (id + fields) before framing.
#[derive(Default)]
pub struct PacketBuilder {
    buf: Vec<u8>,
}

impl PacketBuilder {
    pub fn new(id: i32) -> Self {
        let mut buf = Vec::new();
        write_varint(&mut buf, id);
        Self { buf }
    }

    pub fn varint(mut self, v: i32) -> Self {
        write_varint(&mut self.buf, v);
        self
    }

    pub fn string(mut self, s: &str) -> Self {
        write_string(&mut self.buf, s);
        self
    }

    pub fn u16(mut self, v: u16) -> Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn i64(mut self, v: i64) -> Self {
        self.buf.extend_from_slice(&v.to_be_bytes());
        self
    }

    pub fn body(&self) -> &[u8] {
        &self.buf
    }
}

/// Writes an uncompressed frame: `VarInt length` followed by `id + data`.
pub fn write_frame<W: Write>(w: &mut W, packet: &PacketBuilder) -> Result<()> {
    let body = packet.body();
    let mut frame = Vec::with_capacity(body.len() + 5);
    write_varint(&mut frame, body.len() as i32);
    frame.extend_from_slice(body);
    w.write_all(&frame)?;
    Ok(())
}

/// Reads one uncompressed frame, returning `(packet_id, payload)`.
pub fn read_frame<R: Read>(r: &mut R) -> Result<(i32, Vec<u8>)> {
    let len = read_varint(r)?;
    if len <= 0 || len as usize > MAX_PACKET_LEN {
        bail!("invalid packet length {len}");
    }
    let mut data = vec![0u8; len as usize];
    r.read_exact(&mut data)?;
    let mut cursor = std::io::Cursor::new(data);
    let id = read_varint(&mut cursor)?;
    let pos = cursor.position() as usize;
    let mut data = cursor.into_inner();
    data.drain(..pos);
    Ok((id, data))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_known_values() {
        let cases: &[(i32, &[u8])] = &[
            (0, &[0x00]),
            (1, &[0x01]),
            (127, &[0x7f]),
            (128, &[0x80, 0x01]),
            (255, &[0xff, 0x01]),
            (25565, &[0xdd, 0xc7, 0x01]),
            (2_097_151, &[0xff, 0xff, 0x7f]),
            (i32::MAX, &[0xff, 0xff, 0xff, 0xff, 0x07]),
            (-1, &[0xff, 0xff, 0xff, 0xff, 0x0f]),
            (i32::MIN, &[0x80, 0x80, 0x80, 0x80, 0x08]),
        ];
        for &(value, bytes) in cases {
            let mut buf = Vec::new();
            write_varint(&mut buf, value);
            assert_eq!(buf, bytes, "encoding {value}");
            assert_eq!(
                read_varint(&mut &bytes[..]).unwrap(),
                value,
                "decoding {value}"
            );
        }
    }

    #[test]
    fn varint_too_long() {
        let bytes = [0xff; 6];
        assert!(read_varint(&mut &bytes[..]).is_err());
    }

    #[test]
    fn frame_roundtrip() {
        let mut out = Vec::new();
        write_frame(&mut out, &PacketBuilder::new(0x2a).string("hello").i64(7)).unwrap();
        let (id, payload) = read_frame(&mut &out[..]).unwrap();
        assert_eq!(id, 0x2a);
        let mut r = &payload[..];
        assert_eq!(read_string(&mut r, 16).unwrap(), "hello");
        assert_eq!(r, 7i64.to_be_bytes());
    }
}
