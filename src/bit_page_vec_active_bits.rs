// @author shailendra.sharma
use itertools::{EitherOrBoth, Itertools};
use log::{debug, log_enabled, Level};

use crate::bit_page::{zero_masks, BitPageWithPosition};
use crate::bit_page_vec::BitPageVecKind;
use crate::{BitPage, BitPageVec};

impl BitPageVec {
    pub fn active_bits_count(&self) -> usize {
        match self.kind {
            BitPageVecKind::AllZeroes => 0,
            // bit pages are zero based
            BitPageVecKind::AllOnes => self.last_bit_index.0 * BitPage::MAX_BITS + (self.last_bit_index.1),
            BitPageVecKind::SparseWithZeroesHole => {
                if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
                    debug!(target: "bit_page_vec_log", "active_bits_count(kind=SparseWithZeroesHole) #pages={}", self.size());
                }

                if let Some(ref pages) = self.pages {
                    let last_page = self.last_bit_index.0;
                    let last_bit = self.last_bit_index.1;
                    pages
                        .iter()
                        .filter(move |value| value.page_idx <= last_page)
                        .map(move |value| {
                            if value.page_idx == last_page {
                                let bit_page = value.bit_page & zero_masks()[last_bit];
                                bit_page.count_ones() as usize
                            } else {
                                value.bit_page.count_ones() as usize
                            }
                        })
                        .sum::<usize>()
                } else {
                    0
                }
            }
            BitPageVecKind::SparseWithOnesHole => {
                if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
                    debug!(target: "bit_page_vec_log", "active_bits_count(kind=SparseWithOnesHole) #pages={}", self.size());
                }

                if let Some(ref pages) = self.pages {
                    (0..self.last_bit_index.0)
                        .merge_join_by(pages.iter(), |page_1_idx, BitPageWithPosition { page_idx: page_2_idx, .. }| {
                            page_1_idx.cmp(page_2_idx)
                        })
                        .map(move |either| match either {
                            EitherOrBoth::Both(_, BitPageWithPosition { bit_page, .. }) => bit_page.count_ones() as usize,
                            EitherOrBoth::Left(_) => BitPage::MAX_BITS,
                            EitherOrBoth::Right(BitPageWithPosition { .. }) => 0,
                        })
                        .sum::<usize>()
                        + self.last_bit_index.1
                } else {
                    (0..self.last_bit_index.0).map(|_| BitPage::MAX_BITS).sum::<usize>() + self.last_bit_index.1
                }
            }
        }
    }

    pub fn active_bits(&self) -> BitPageVecActiveBitsIterator {
        match self.kind {
            BitPageVecKind::AllZeroes => BitPageVecActiveBitsIterator::None,
            BitPageVecKind::AllOnes => {
                let iter = (0..self.last_bit_index.0)
                    .flat_map(|page_idx| BitPage::active_bits(BitPage::ones()).map(move |bit_idx| (page_idx, bit_idx)))
                    .chain(
                        BitPage::active_bits(BitPage::ones())
                            .filter(move |bit_idx| bit_idx.lt(&self.last_bit_index.1))
                            .map(move |bit_idx| (self.last_bit_index.0, bit_idx)),
                    );

                BitPageVecActiveBitsIterator::Some { iter: Box::new(iter) }
            }
            BitPageVecKind::SparseWithZeroesHole => {
                if let Some(ref pages) = self.pages {
                    let last_page = self.last_bit_index.0;
                    let last_bit = self.last_bit_index.1;
                    let iter = pages.iter().filter(move |value| value.page_idx <= last_page).flat_map(
                        move |BitPageWithPosition { page_idx, bit_page }| {
                            BitPage::active_bits(*bit_page)
                                .filter(move |bit_idx| page_idx.lt(&last_page) || bit_idx.lt(&last_bit))
                                .map(move |bit_idx| (*page_idx, bit_idx))
                        },
                    );

                    BitPageVecActiveBitsIterator::Some { iter: Box::new(iter) }
                } else {
                    BitPageVecActiveBitsIterator::None
                }
            }
            BitPageVecKind::SparseWithOnesHole => {
                if let Some(ref pages) = self.pages {
                    let iter = (0..=self.last_bit_index.0)
                        .merge_join_by(pages.iter(), |page_1_idx, BitPageWithPosition { page_idx: page_2_idx, .. }| {
                            page_1_idx.cmp(page_2_idx)
                        })
                        .flat_map(move |either| match either {
                            EitherOrBoth::Both(_, BitPageWithPosition { page_idx, bit_page }) => {
                                let iter: Box<dyn Iterator<Item = (usize, usize)>> =
                                    Box::new(BitPage::active_bits(*bit_page).map(move |bit_idx| (*page_idx, bit_idx)));
                                iter
                            }
                            EitherOrBoth::Left(page_idx) => {
                                if page_idx.eq(&self.last_bit_index.0) {
                                    let bit_page = BitPage::ones();
                                    let iter: Box<dyn Iterator<Item = (usize, usize)>> = Box::new(
                                        BitPage::active_bits(bit_page)
                                            .filter(move |bit_idx| bit_idx.lt(&self.last_bit_index.1))
                                            .map(move |bit_idx| (page_idx, bit_idx)),
                                    );
                                    iter
                                } else {
                                    let bit_page = BitPage::ones();
                                    let iter: Box<dyn Iterator<Item = (usize, usize)>> =
                                        Box::new(BitPage::active_bits(bit_page).map(move |bit_idx| (page_idx, bit_idx)));
                                    iter
                                }
                            }
                            EitherOrBoth::Right(BitPageWithPosition { page_idx, .. }) => {
                                let bit_page = BitPage::zeroes();
                                let iter: Box<dyn Iterator<Item = (usize, usize)>> =
                                    Box::new(BitPage::active_bits(bit_page).map(move |bit_idx| (*page_idx, bit_idx)));
                                iter
                            }
                        });

                    BitPageVecActiveBitsIterator::Some { iter: Box::new(iter) }
                } else {
                    // duplicate of AllOnes case
                    let iter = (0..self.last_bit_index.0)
                        .flat_map(|page_idx| BitPage::active_bits(BitPage::ones()).map(move |bit_idx| (page_idx, bit_idx)))
                        .chain(
                            BitPage::active_bits(BitPage::ones())
                                .filter(move |bit_idx| bit_idx.lt(&self.last_bit_index.1))
                                .map(move |bit_idx| (self.last_bit_index.0, bit_idx)),
                        );

                    BitPageVecActiveBitsIterator::Some { iter: Box::new(iter) }
                }
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

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::BitPageVec;

    #[test]
    fn test_bit_page_active_bits() {
        let last_page = 0;
        let last_bit = 1;
        let mut bit_page_vec = BitPageVec::all_zeros((last_page, last_bit));

        for page in 0..2 {
            for bit in 0..4 {
                bit_page_vec.set_bit(page, bit);
            }
        }

        println!("Vector = {:?}", bit_page_vec);
        println!("Active Bits Count = {}", bit_page_vec.active_bits_count());
        println!("Active Bits = {:?}", bit_page_vec.active_bits().collect_vec());
    }
}
