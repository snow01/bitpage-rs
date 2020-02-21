// @shailendra.sharma
use bytes::{Buf, BufMut};

use crate::bit_page_vec::BitPageWithPosition;
use crate::{BitPage, BitPageVec};

impl BitPageVec {
    pub fn encode<W>(&self, buf: &mut W) -> anyhow::Result<()>
    where
        W: BufMut,
    {
        match self {
            BitPageVec::AllZeroes => {
                // write type
                buf.put_u8(0);
            }
            BitPageVec::SparseWithZeroesHole(pages) => {
                // write type
                buf.put_u8(1);

                Self::encode_pages(pages, buf);
            }
            _ => anyhow::bail!("Not a valid BitPageVec type for encoding={:?}", self),
            // BitPageVec::AllOnes => {
            //     buf.put_u8(2);
            // }
            // BitPageVec::SparseWithOnesHole(pages) => {
            //     buf.put_u8(3);
            //
            //     Self::encode_pages(pages, buf);
            // }
        }

        Ok(())
    }

    fn encode_pages<W>(pages: &[BitPageWithPosition], buf: &mut W)
    where
        W: BufMut,
    {
        // write length
        buf.put_u64(pages.len() as u64);

        for page in pages {
            buf.put_u64(page.page_idx as u64);

            // depending on the value encode here...
            BitPage::encode(page.bit_page, buf);
        }
    }

    pub fn decode<R>(buf: &mut R) -> anyhow::Result<BitPageVec>
    where
        R: Buf,
    {
        anyhow::ensure!(buf.has_remaining(), "No more bytes remaining to decode to BitPageVec");

        let t = buf.get_u8();
        match t {
            0 => Ok(BitPageVec::AllZeroes),
            1 => {
                let pages = Self::decode_pages(buf)?;

                Ok(BitPageVec::SparseWithZeroesHole(pages))
            }
            // 2 => Ok(BitPageVec::AllOnes),
            // 3 => {
            //     let pages = Self::decode_pages(buf)?;
            //
            //     Ok(Self::compact_sparse_with_ones_hole(pages))
            // }
            _ => anyhow::bail!("Not a valid BitPageVec type={} for decoding", t),
        }
    }

    pub(crate) fn decode_pages<R>(buf: &mut R) -> anyhow::Result<Vec<BitPageWithPosition>>
    where
        R: Buf,
    {
        anyhow::ensure!(buf.remaining() >= 8, "BitPageVec: No more bytes remaining to decode pages length");

        let length = buf.get_u64() as usize;

        let mut pages = Vec::with_capacity(length);

        for _ in 0..length {
            anyhow::ensure!(buf.remaining() >= 8, "BitPageVec: No more bytes remaining to decode pages length");

            let page_idx = buf.get_u64() as usize;

            let bit_page = BitPage::decode(buf)?;

            pages.push(BitPageWithPosition { page_idx, bit_page });
        }

        Ok(pages)
    }
}
