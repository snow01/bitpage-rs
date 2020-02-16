use bytes::{Buf, BufMut};

use crate::BitPage;

impl BitPage {
    pub fn encode<W>(&self, buf: &mut W)
    where
        W: BufMut,
    {
        match self {
            BitPage::Zeroes => {
                buf.put_u8(0);
            }
            BitPage::Ones => {
                buf.put_u8(1);
            }
            BitPage::Some(value) => {
                buf.put_u8(2);
                buf.put_u64(*value);
            }
        }
    }

    pub fn decode<R>(buf: &mut R) -> anyhow::Result<BitPage>
    where
        R: Buf,
    {
        anyhow::ensure!(buf.has_remaining(), "No more bytes remaining to decode to BitPage");

        let t = buf.get_u8();
        if t == 0 {
            Ok(BitPage::Zeroes)
        } else if t == 1 {
            Ok(BitPage::Ones)
        } else {
            anyhow::ensure!(buf.remaining() >= 8, "No more bytes remaining to decode to BitPage value");
            let value = buf.get_u64();
            Ok(BitPage::Some(value))
        }
    }
}
