use bytes::{Buf, BufMut};

use crate::bit_page_vec::BitPageWithPosition;
use crate::{BitPage, BitPageVec};

impl BitPageVec {
    pub fn encode<W>(&self, buf: &mut W)
    where
        W: BufMut,
    {
        match self {
            BitPageVec::AllZeroes => {
                // write type
                buf.put_u8(0);
            }
            BitPageVec::Sparse(pages) => {
                // write type
                buf.put_u8(1);

                // write length
                buf.put_u64(pages.len() as u64);

                for page in pages {
                    buf.put_u64(page.page_idx as u64);
                    page.bit_page.encode(buf);
                }
            }
        }
    }

    pub fn decode<R>(buf: &mut R) -> anyhow::Result<BitPageVec>
    where
        R: Buf,
    {
        anyhow::ensure!(buf.has_remaining(), "No more bytes remaining to decode to BitPageVec");

        let t = buf.get_u8();
        if t == 0 {
            Ok(BitPageVec::AllZeroes)
        } else {
            anyhow::ensure!(buf.remaining() >= 8, "BitPageVec: No more bytes remaining to decode pages length");

            let length = buf.get_u64() as usize;

            let mut pages = Vec::with_capacity(length);

            for _ in 0..length {
                anyhow::ensure!(buf.remaining() >= 8, "BitPageVec: No more bytes remaining to decode pages length");

                let page_idx = buf.get_u64() as usize;
                let bit_page = BitPage::decode(buf)?;

                pages.push(BitPageWithPosition { page_idx, bit_page });
            }

            Ok(BitPageVec::Sparse(pages))
        }
    }
}
