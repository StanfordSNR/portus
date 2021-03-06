//! Message sent from datapath to CCP when a new flow starts.

use std::io::prelude::*;
use Result;
use super::{AsRawMsg, RawMsg, HDR_LENGTH, u32_to_u8s};

pub(crate) const CREATE: u8 = 0;

#[derive(Clone, Copy)]
#[derive(Debug)]
#[derive(PartialEq)]
pub struct Msg {
    pub sid: u32,
    pub init_cwnd: u32,
    pub mss: u32,
    pub src_ip: u32,
    pub src_port: u32,
    pub dst_ip: u32,
    pub dst_port: u32,
}

impl AsRawMsg for Msg {
    fn get_hdr(&self) -> (u8, u32, u32) {
        (
            CREATE,
            HDR_LENGTH + 6 * 4,
            self.sid,
        )
    }

    fn get_u32s<W: Write>(&self, w: &mut W) -> Result<()> {
        let mut buf = [0u8; 4];
        u32_to_u8s(&mut buf, self.init_cwnd as u32);
        w.write_all(&buf[..])?;
        u32_to_u8s(&mut buf, self.mss as u32);
        w.write_all(&buf[..])?;
        u32_to_u8s(&mut buf, self.src_ip as u32);
        w.write_all(&buf[..])?;
        u32_to_u8s(&mut buf, self.src_port as u32);
        w.write_all(&buf[..])?;
        u32_to_u8s(&mut buf, self.dst_ip as u32);
        w.write_all(&buf[..])?;
        u32_to_u8s(&mut buf, self.dst_port as u32);
        w.write_all(&buf[..])?;
        Ok(())
    }

    fn get_bytes<W: Write>(&self, _: &mut W) -> Result<()> {
        Ok(())
    }

    fn from_raw_msg(msg: RawMsg) -> Result<Self> {
        let u32s = unsafe { msg.get_u32s() }?;
        Ok(Msg {
            sid: msg.sid,
            init_cwnd: u32s[0],
            mss: u32s[1],
            src_ip: u32s[2],
            src_port: u32s[3],
            dst_ip: u32s[4],
            dst_port: u32s[5],
        })
    }
}

#[cfg(test)]
mod tests {
    macro_rules! check_create_msg {
        ($id: ident, $msg: expr) => (
            check_msg!(
                $id, 
                super::Msg,
                $msg,
                ::serialize::Msg::Cr(crm),
                crm
            );
        )
    }

    check_create_msg!(
        test_create_1,
        super::Msg{
            sid: 15,
            init_cwnd: 1448 * 10,
            mss: 1448,
            src_ip: 0,
            src_port: 4242,
            dst_ip: 0,
            dst_port: 4242,
        }
    );

    extern crate test;
    use self::test::Bencher;

    #[bench]
    fn bench_flip_create(b: &mut Bencher) {
        b.iter(|| test_create_1())
    }
}
