// @author shailendra.sharma
use std::cmp::min;

use itertools::{EitherOrBoth, Itertools};
use log::{debug, log_enabled, Level};

use crate::bit_page_vec::BitPageWithPosition;
use crate::{BitPage, BitPageVec};

impl BitPageVec {
    pub fn active_bits_count(&self, num_pages: usize) -> usize {
        match self {
            BitPageVec::AllZeroes => 0,
            BitPageVec::AllOnes => num_pages * BitPage::MAX_BITS,
            BitPageVec::SparseWithZeroesHole(pages) => {
                if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
                    debug!(target: "bit_page_vec_log", "active_bits_count(kind=SparseWithZeroesHole) #pages={}, pages={:?}", pages.len(), pages);
                }

                BitPageVec::count_ones(pages) as usize
            }
            BitPageVec::SparseWithOnesHole(pages) => {
                if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
                    debug!(target: "bit_page_vec_log", "active_bits_count(kind=SparseWithOnesHole) #pages={}, pages={:?}", pages.len(), pages);
                }

                BitPageVec::count_ones(pages) as usize + (num_pages - min(pages.len(), num_pages)) * BitPage::MAX_BITS
            }
        }
    }

    pub fn active_bits(&self, num_pages: usize) -> BitPageVecActiveBitsIterator {
        match self {
            BitPageVec::AllZeroes => BitPageVecActiveBitsIterator::None,
            BitPageVec::AllOnes => {
                let iter =
                    (0..num_pages).flat_map(|page_idx| BitPage::active_bits(BitPage::ones()).map(move |bit_idx| (page_idx, bit_idx)));

                BitPageVecActiveBitsIterator::Some { iter: Box::new(iter) }
            }
            BitPageVec::SparseWithZeroesHole(pages) => {
                let iter = pages.iter().flat_map(|BitPageWithPosition { page_idx, bit_page }| {
                    BitPage::active_bits(*bit_page).map(move |bit_idx| (*page_idx, bit_idx))
                });

                BitPageVecActiveBitsIterator::Some { iter: Box::new(iter) }
            }
            BitPageVec::SparseWithOnesHole(pages) => {
                let iter = (0..num_pages)
                    .merge_join_by(pages.iter(), |page_1_idx, BitPageWithPosition { page_idx: page_2_idx, .. }| {
                        page_1_idx.cmp(page_2_idx)
                    })
                    .flat_map(|either| match either {
                        EitherOrBoth::Both(_, BitPageWithPosition { page_idx, bit_page }) => {
                            let iter: Box<dyn Iterator<Item = (usize, usize)>> =
                                Box::new(BitPage::active_bits(*bit_page).map(move |bit_idx| (*page_idx, bit_idx)));
                            iter
                        }
                        EitherOrBoth::Left(page_idx) => {
                            let bit_page = BitPage::ones();
                            let iter: Box<dyn Iterator<Item = (usize, usize)>> =
                                Box::new(BitPage::active_bits(bit_page).map(move |bit_idx| (page_idx, bit_idx)));
                            iter
                        }
                        EitherOrBoth::Right(BitPageWithPosition { page_idx, .. }) => {
                            let bit_page = BitPage::zeroes();
                            let iter: Box<dyn Iterator<Item = (usize, usize)>> =
                                Box::new(BitPage::active_bits(bit_page).map(move |bit_idx| (*page_idx, bit_idx)));
                            iter
                        }
                    });

                BitPageVecActiveBitsIterator::Some { iter: Box::new(iter) }
            }
        }
    }
}

pub enum BitPageVecActiveBitsIterator<'a> {
    None,
    Some {
        iter: Box<dyn Iterator<Item = (usize, usize)> + 'a>,
    },
}

impl<'a> Iterator for BitPageVecActiveBitsIterator<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            BitPageVecActiveBitsIterator::None => None,
            BitPageVecActiveBitsIterator::Some { iter } => iter.next(),
        }
    }
}
