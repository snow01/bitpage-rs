use bytes::{Buf, BufMut};
use itertools::{EitherOrBoth, Itertools};
use log::{debug, log_enabled, Level};

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
            BitPageVec::SparseWithZeroesHole(pages) => {
                // write type
                buf.put_u8(1);

                Self::encode_pages(pages, buf);
            }
            BitPageVec::AllOnes => {
                buf.put_u8(2);
            }
            BitPageVec::SparseWithOnesHole(pages) => {
                buf.put_u8(3);

                Self::encode_pages(pages, buf);
            }
        }
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

                Ok(Self::compact_sparse_with_zeroes_hole(pages))
            }
            2 => Ok(BitPageVec::AllOnes),
            3 => {
                let pages = Self::decode_pages(buf)?;

                Ok(Self::compact_sparse_with_ones_hole(pages))
            }
            _ => anyhow::bail!("Not a valid BitPageVec type={}", t),
        }
    }

    pub(crate) fn compact_sparse_with_zeroes_hole(pages: Vec<BitPageWithPosition>) -> BitPageVec {
        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "compact_sparse_with_zeroes_hole - pages len={}", pages.len());
        }

        let result = if pages.is_empty() {
            BitPageVec::AllZeroes
        } else if pages.len() <= 10_000 {
            BitPageVec::SparseWithZeroesHole(pages)
        } else {
            let start_page = pages[0].page_idx;
            let end_page = pages[pages.len() - 1].page_idx;
            let max_possible_length = (end_page - start_page + 1) as f64;
            let actual_length = pages.len() as f64;

            if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
                debug!(target: "bit_page_vec_log", "compact_sparse_with_zeroes_hole - start_page={} end_page={} max_possible_length={} actual_length={}", start_page, end_page, max_possible_length, actual_length);
            }

            // find start page, end page, and length
            // if length >= 75% of (end - start) page
            // and # of active bits >= 75% of active bits needed for fully packed 75%
            if actual_length >= 0.75 * max_possible_length && BitPageVec::count_ones(&pages) as f64 >= 0.75 * max_possible_length * 64.0 {
                if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
                    debug!(target: "bit_page_vec_log", "compact_sparse_with_zeroes_hole::compacting - ones={}", BitPageVec::count_ones(&pages));
                }

                // filter out all page with max value
                // and include pages with holes
                let pages = (0..=end_page)
                    .merge_join_by(pages.into_iter(), |page_1_idx, BitPageWithPosition { page_idx: page_2_idx, .. }| {
                        page_1_idx.cmp(page_2_idx)
                    })
                    .filter_map(|either| {
                        match either {
                            EitherOrBoth::Both(_, BitPageWithPosition { page_idx, bit_page }) => {
                                if BitPage::is_ones(&bit_page) {
                                    None
                                } else {
                                    Some(BitPageWithPosition { page_idx, bit_page })
                                }
                            }
                            EitherOrBoth::Left(page_idx) => Some(BitPageWithPosition {
                                page_idx,
                                bit_page: BitPage::zeroes(),
                            }),
                            EitherOrBoth::Right(_) => {
                                // this case should not arise
                                None
                            }
                        }
                    })
                    .collect_vec();

                BitPageVec::SparseWithOnesHole(pages)
            } else {
                BitPageVec::SparseWithZeroesHole(pages)
            }
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "compact_sparse_with_zeroes_hole::result={:?}", result);
        }

        result
    }

    pub(crate) fn compact_sparse_with_ones_hole(pages: Vec<BitPageWithPosition>) -> BitPageVec {
        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "compact_sparse_with_ones_hole - pages len={}", pages.len());
        }

        let result = if pages.is_empty() {
            BitPageVec::AllOnes
        } else if pages.len() <= 10_000 {
            BitPageVec::SparseWithOnesHole(pages)
        } else {
            let start_page = pages[0].page_idx;
            let end_page = pages[pages.len() - 1].page_idx;
            let max_possible_length = (end_page - start_page + 1) as f64;
            let actual_length = pages.len() as f64;

            if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
                debug!(target: "bit_page_vec_log", "compact_sparse_with_ones_hole - start_page={} end_page={} max_possible_length={} actual_length={}", start_page, end_page, max_possible_length, actual_length);
            }

            // find start page, end page, and length
            // if length >= 75% of (end - start) page
            // and # of active bits <= 25% of active bits needed for fully packed 75%
            if actual_length >= 0.75 * max_possible_length && BitPageVec::count_ones(&pages) as f64 <= 0.25 * max_possible_length * 64.0 {
                if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
                    debug!(target: "bit_page_vec_log", "compact_sparse_with_ones_hole::compacting - ones={}", BitPageVec::count_ones(&pages));
                }

                // filter out all page with max value
                // and include pages with holes
                let pages = (0..=end_page)
                    .merge_join_by(pages.into_iter(), |page_1_idx, BitPageWithPosition { page_idx: page_2_idx, .. }| {
                        page_1_idx.cmp(page_2_idx)
                    })
                    .filter_map(|either| {
                        match either {
                            EitherOrBoth::Both(_, BitPageWithPosition { page_idx, bit_page }) => {
                                if BitPage::is_zeroes(&bit_page) {
                                    None
                                } else {
                                    Some(BitPageWithPosition { page_idx, bit_page })
                                }
                            }
                            EitherOrBoth::Left(page_idx) => Some(BitPageWithPosition {
                                page_idx,
                                bit_page: BitPage::ones(),
                            }),
                            EitherOrBoth::Right(_) => {
                                // this case should not arise
                                None
                            }
                        }
                    })
                    .collect_vec();

                BitPageVec::SparseWithZeroesHole(pages)
            } else {
                BitPageVec::SparseWithOnesHole(pages)
            }
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "compact_sparse_with_ones_hole::result={:?}", result);
        }

        result
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
