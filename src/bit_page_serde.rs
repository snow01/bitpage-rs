use bytes::{Buf, BufMut};

use crate::BitPage;

// TODO: this is for backward compatibility of indices... as they gets changed... we can only encode u64 directly

const MAX_VALUE: u64 = u64::max_value();

impl BitPage {
    pub fn encode<W>(value: u64, buf: &mut W)
    where
        W: BufMut,
    {
        match value {
            0 => {
                buf.put_u8(0);
            }
            MAX_VALUE => {
                buf.put_u8(1);
            }
            _ => {
                buf.put_u8(2);
                buf.put_u64(value);
            }
        }
    }

    pub fn decode<R>(buf: &mut R) -> anyhow::Result<u64>
    where
        R: Buf,
    {
        anyhow::ensure!(buf.has_remaining(), "No more bytes remaining to decode to BitPage");

        let t = buf.get_u8();
        if t == 0 {
            Ok(0)
        } else if t == 1 {
            Ok(MAX_VALUE)
        } else {
            anyhow::ensure!(buf.remaining() >= 8, "No more bytes remaining to decode to BitPage value");
            let value = buf.get_u64();
            Ok(value)
        }
    }
}
