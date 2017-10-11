use std;
use std::vec::Vec;
use std::io::prelude::*;
use std::io::Cursor;

use super::Error;
use super::Result;

use bytes::{ByteOrder, LittleEndian};

pub(crate) fn u32_to_u8s(buf: &mut [u8], num: u32) {
    LittleEndian::write_u32(buf, num);
}

pub(crate) fn u64_to_u8s(buf: &mut [u8], num: u64) {
    LittleEndian::write_u64(buf, num);
}

pub(crate) fn u32_from_u8s(buf: &[u8]) -> u32 {
    LittleEndian::read_u32(buf)
}

//pub(crate) fn u64_from_u8s(buf: &[u8]) -> u64 {
//    LittleEndian::read_u64(buf)
//}

/* (type, len, socket_id) header
 * -----------------------------------
 * | Msg Type | Len (B)  | Uint32    |
 * | (1 B)    | (1 B)    | (32 bits) |
 * -----------------------------------
 * total: 6 Bytes
 */
const HDR_LENGTH: u8 = 6;
fn serialize_header(typ: u8, len: u8, sid: u32) -> Vec<u8> {
    let mut hdr = Vec::new();
    hdr.push(typ);
    hdr.push(len);
    let mut buf = [0u8; 4];
    u32_to_u8s(&mut buf, sid);
    hdr.extend(&buf[..]);
    hdr
}

fn deserialize_header<R: Read>(buf: &mut R) -> Result<(u8, u8, u32)> {
    let mut hdr = [0u8; 6];
    buf.read_exact(&mut hdr)?;
    let typ = hdr[0];
    let len = hdr[1];
    let sid = u32_from_u8s(&hdr[2..]);

    Ok((typ, len, sid))
}

pub(crate) struct RawMsg<'a> {
    typ: u8,
    len: u8,
    sid: u32,
    bytes: &'a [u8],
}

impl<'a> RawMsg<'a> {
    pub(crate) unsafe fn get_u32s(&self) -> Result<&'a [u32]> {
        use std::mem;
        match self.typ {
            CREATE => Ok(mem::transmute(&self.bytes[0..4])),
            MEASURE => Ok(mem::transmute(&self.bytes[0..4 * 2])),
            DROP => Ok(&[]),
            CWND => Ok(&[]),
            _ => Err(Error(String::from("malformed msg"))),
        }
    }

    pub(crate) unsafe fn get_u64s(&self) -> Result<&'a [u64]> {
        use std::mem;
        match self.typ {
            CREATE => Ok(&[]),
            MEASURE => Ok(mem::transmute(&self.bytes[(4 * 2)..(4 * 2 + 8 * 2)])),
            DROP => Ok(&[]),
            CWND => Ok(&[]),
            _ => Err(Error(String::from("malformed msg"))),
        }
    }

    pub(crate) fn get_bytes(&self) -> Result<&'a [u8]> {
        match self.typ {
            CREATE => Ok(&self.bytes[4..(self.len as usize - 6)]),
            MEASURE => Ok(&[]),
            DROP => Ok(&self.bytes[0..(self.len as usize - 6)]),
            CWND => Ok(&self.bytes[0..(self.len as usize - 6)]),
            _ => Err(Error(String::from("malformed msg"))),
        }
    }
}

pub(crate) trait AsRawMsg {
    fn get_hdr(&self) -> (u8, u8, u32);
    fn get_u32s<W: Write>(&self, w: &mut W) -> Result<()>;
    fn get_u64s<W: Write>(&self, w: &mut W) -> Result<()>;
    fn get_bytes<W: Write>(&self, w: &mut W) -> Result<()>;

    fn from_raw_msg(msg: RawMsg) -> Result<Self>
    where
        Self: std::marker::Sized;
}

pub(crate) struct RMsg<T: AsRawMsg>(pub T);

impl<T: AsRawMsg> RMsg<T> {
    pub fn serialize(&self) -> Result<Vec<u8>> {
        let (a, b, c) = self.0.get_hdr();
        let mut msg = serialize_header(a, b, c);
        self.0.get_u32s(&mut msg)?;
        self.0.get_u64s(&mut msg)?;
        self.0.get_bytes(&mut msg)?;
        Ok(msg)
    }
}

const CREATE: u8 = 0;
#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub struct CreateMsg {
    pub sid: u32,
    pub start_seq: u32,
    pub cong_alg: String,
}

impl AsRawMsg for CreateMsg {
    fn get_hdr(&self) -> (u8, u8, u32) {
        (CREATE, HDR_LENGTH + 4 + self.cong_alg.len() as u8, self.sid)
    }

    fn get_u32s<W: Write>(&self, w: &mut W) -> Result<()> {
        let mut buf = [0u8; 4];
        u32_to_u8s(&mut buf, self.start_seq);
        w.write_all(&buf[..])?;
        Ok(())
    }

    fn get_u64s<W: Write>(&self, _: &mut W) -> Result<()> {
        Ok(())
    }

    fn get_bytes<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_all(self.cong_alg.clone().as_bytes())?;
        Ok(())
    }

    fn from_raw_msg(msg: RawMsg) -> Result<Self> {
        let b = msg.get_bytes()?;
        let s = std::str::from_utf8(b)?;
        let alg = String::from(s);
        Ok(CreateMsg {
            sid: msg.sid,
            start_seq: unsafe { msg.get_u32s() }?[0],
            cong_alg: alg,
        })
    }
}

const MEASURE: u8 = 1;
#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub struct MeasureMsg {
    pub sid: u32,
    pub ack: u32,
    pub rtt_us: u32,
    pub rin: u64,
    pub rout: u64,
}

impl AsRawMsg for MeasureMsg {
    fn get_hdr(&self) -> (u8, u8, u32) {
        (MEASURE, HDR_LENGTH + 8 + 16 as u8, self.sid)
    }

    fn get_u32s<W: Write>(&self, w: &mut W) -> Result<()> {
        let mut buf = [0u8; 4];
        u32_to_u8s(&mut buf, self.ack);
        w.write_all(&buf[..])?;
        u32_to_u8s(&mut buf, self.rtt_us);
        w.write_all(&buf[..])?;
        Ok(())
    }

    fn get_u64s<W: Write>(&self, w: &mut W) -> Result<()> {
        let mut buf = [0u8; 8];
        u64_to_u8s(&mut buf, self.rin);
        w.write_all(&buf[..])?;
        u64_to_u8s(&mut buf, self.rout);
        w.write_all(&buf[..])?;
        Ok(())
    }

    fn get_bytes<W: Write>(&self, _: &mut W) -> Result<()> {
        Ok(())
    }

    fn from_raw_msg(msg: RawMsg) -> Result<Self> {
        let u32s = unsafe { msg.get_u32s() }?;
        let u64s = unsafe { msg.get_u64s() }?;
        Ok(MeasureMsg {
            sid: msg.sid,
            ack: u32s[0],
            rtt_us: u32s[1],
            rin: u64s[0],
            rout: u64s[1],
        })
    }
}

#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub struct DropMsg {
    pub sid: u32,
    pub event: String,
}

const DROP: u8 = 2;
impl AsRawMsg for DropMsg {
    fn get_hdr(&self) -> (u8, u8, u32) {
        (DROP, HDR_LENGTH + self.event.len() as u8, self.sid)
    }

    fn get_u32s<W: Write>(&self, _: &mut W) -> Result<()> {
        Ok(())
    }

    fn get_u64s<W: Write>(&self, _: &mut W) -> Result<()> {
        Ok(())
    }

    fn get_bytes<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_all(self.event.clone().as_bytes())?;
        Ok(())
    }

    fn from_raw_msg(msg: RawMsg) -> Result<Self> {
        let b = msg.get_bytes()?;
        let s = std::str::from_utf8(b)?;
        let ev = String::from(s);
        Ok(DropMsg {
            sid: msg.sid,
            event: ev,
        })
    }
}

use super::pattern;
#[derive(Clone)]
#[derive(Debug)]
#[derive(PartialEq)]
pub struct PatternMsg {
    pub sid: u32,
    pub pattern: pattern::Pattern,
}

const CWND: u8 = 3;
impl AsRawMsg for PatternMsg {
    fn get_hdr(&self) -> (u8, u8, u32) {
        (CWND, HDR_LENGTH + self.pattern.len_bytes() as u8, self.sid)
    }

    fn get_u32s<W: Write>(&self, _: &mut W) -> Result<()> {
        Ok(())
    }

    fn get_u64s<W: Write>(&self, _: &mut W) -> Result<()> {
        Ok(())
    }

    fn get_bytes<W: Write>(&self, w: &mut W) -> Result<()> {
        self.pattern.serialize(w)?;
        Ok(())
    }

    fn from_raw_msg(msg: RawMsg) -> Result<Self> {
        let mut b = msg.get_bytes()?;
        Ok(PatternMsg {
            sid: msg.sid,
            pattern: pattern::Pattern::deserialize(&mut b)?,
        })
    }
}

fn deserialize(buf: &[u8]) -> Result<RawMsg> {
    let mut buf = Cursor::new(buf);
    let (typ, len, sid) = deserialize_header(&mut buf)?;
    let i = buf.position();
    Ok(RawMsg {
        typ: typ,
        len: len,
        sid: sid,
        bytes: &buf.into_inner()[i as usize..],
    })
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum Msg {
    Cr(CreateMsg),
    Dr(DropMsg),
    Ms(MeasureMsg),
    Pt(PatternMsg),
}

impl Msg {
    fn from_raw_msg(m: RawMsg) -> Result<Msg> {
        match m.typ {
            CREATE => Ok(Msg::Cr(CreateMsg::from_raw_msg(m)?)),
            DROP => Ok(Msg::Dr(DropMsg::from_raw_msg(m)?)),
            MEASURE => Ok(Msg::Ms(MeasureMsg::from_raw_msg(m)?)),
            CWND => Ok(Msg::Pt(PatternMsg::from_raw_msg(m)?)),
            _ => Err(Error(String::from("unknown type"))),
        }
    }

    pub fn from_buf(buf: &[u8]) -> Result<Msg> {
        deserialize(buf).and_then(Msg::from_raw_msg)
    }
}

#[cfg(test)]
mod test;
