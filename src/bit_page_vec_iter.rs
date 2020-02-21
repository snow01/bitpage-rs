// @author shailendra.sharma
use std::cmp::{min, Ordering};
use std::fmt;
use std::iter::empty;
use std::time::Instant;

use itertools::{EitherOrBoth, Itertools};
use log::{debug, log_enabled, trace, Level};

use crate::bit_page::BitPageWithPosition;
use crate::bit_page_vec::BitPageVecKind;
use crate::{BitPage, BitPageVec, DbBitPageVec};

pub type PageItem = (usize, u64);
pub type PageIterator<'a> = Box<dyn Iterator<Item = PageItem> + 'a>;

pub struct BitPageVecIter<'a> {
    kind: BitPageVecKind,
    iter: PageIterator<'a>,
    last_bit_index: (usize, usize),
}

impl<'a> fmt::Debug for BitPageVecIter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BitPageVecIter(kind={:?})", self.kind)
    }
}

impl<'a> BitPageVecIter<'a> {
    pub fn new(kind: BitPageVecKind, iter: PageIterator, last_bit_index: (usize, usize)) -> BitPageVecIter {
        BitPageVecIter {
            kind,
            iter,
            last_bit_index,
        }
    }

    pub fn kind(&self) -> &BitPageVecKind {
        &self.kind
    }

    pub fn into_bit_page_vec(self) -> BitPageVec {
        let instant = Instant::now();
        let kind = self.kind;

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "into_bit_page_vec(kind={:?})", self.kind);
        }

        let result = match self.kind {
            BitPageVecKind::AllZeroes => BitPageVec::all_zeros(self.last_bit_index),
            BitPageVecKind::SparseWithZeroesHole => {
                let pages = self
                    .iter
                    .filter_map(|(page_idx, bit_page)| {
                        if BitPage::is_zeroes(&bit_page) {
                            None
                        } else {
                            Some(BitPageWithPosition { page_idx, bit_page })
                        }
                    })
                    .collect_vec();

                Self::compact_sparse_with_zeroes_hole(pages, self.last_bit_index)
            }
            BitPageVecKind::AllOnes => BitPageVec::all_ones(self.last_bit_index),
            BitPageVecKind::SparseWithOnesHole => {
                let pages = self
                    .iter
                    .filter_map(|(page_idx, bit_page)| {
                        if BitPage::is_ones(&bit_page) {
                            None
                        } else {
                            Some(BitPageWithPosition { page_idx, bit_page })
                        }
                    })
                    .collect_vec();

                Self::compact_sparse_with_ones_hole(pages, self.last_bit_index)
            }
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "into_bit_page_vec(kind={:?}):: time taken={:?} and result={:?}", kind, instant.elapsed(), result);
        }

        result
    }

    pub fn add(self, db_value: DbBitPageVec) -> BitPageVecIter<'a> {
        let bit_page_vec = match db_value {
            DbBitPageVec::AllZeroes => BitPageVec::all_zeros(self.last_bit_index),
            DbBitPageVec::Sparse(pages) => BitPageVec::new(BitPageVecKind::SparseWithZeroesHole, Some(pages), self.last_bit_index),
        };

        BitPageVecIter::or(self, bit_page_vec.into_iter())
    }

    pub fn not(self) -> BitPageVecIter<'a> {
        match self.kind {
            BitPageVecKind::AllZeroes => BitPageVec::all_ones(self.last_bit_index).into_iter(),
            BitPageVecKind::SparseWithZeroesHole => BitPageVecIter::new(
                BitPageVecKind::SparseWithOnesHole,
                Box::new(self.iter.map(|(page_idx, bit_page)| (page_idx, !bit_page))),
                self.last_bit_index,
            ),
            BitPageVecKind::AllOnes => BitPageVec::all_zeros(self.last_bit_index).into_iter(),
            BitPageVecKind::SparseWithOnesHole => BitPageVecIter::new(
                BitPageVecKind::SparseWithZeroesHole,
                Box::new(self.iter.map(|(page_idx, bit_page)| (page_idx, !bit_page))),
                self.last_bit_index,
            ),
        }
    }

    pub fn or(first: BitPageVecIter<'a>, second: BitPageVecIter<'a>) -> BitPageVecIter<'a> {
        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "BitPageVecIter::OR first={:?} second={:?}", first, second);
        }

        let result = match first.kind {
            BitPageVecKind::AllZeroes => second,
            BitPageVecKind::SparseWithZeroesHole => match second.kind {
                BitPageVecKind::AllZeroes => first,
                BitPageVecKind::SparseWithZeroesHole => {
                    // merge here... same type with zeroes hole
                    // 0 | 0 => 0
                    // some | 0 => some
                    // 0 | some => some
                    // some | some => or(some)
                    let iter = first.iter.merge_join_by(second.iter, merge_cmp).map(|either| match either {
                        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
                            page_one |= page_two;

                            (idx_1, page_one)
                        }
                        EitherOrBoth::Left((idx, page)) | EitherOrBoth::Right((idx, page)) => (idx, page),
                    });

                    BitPageVecIter::new(
                        BitPageVecKind::SparseWithZeroesHole,
                        Box::new(iter),
                        min_last_bit_index(first.last_bit_index, second.last_bit_index),
                    )
                }
                BitPageVecKind::AllOnes => second,
                BitPageVecKind::SparseWithOnesHole => {
                    // merge here... cross type
                    let iter = first.iter.merge_join_by(second.iter, merge_cmp).filter_map(or_merge_cross_types);
                    BitPageVecIter::new(
                        BitPageVecKind::SparseWithOnesHole,
                        Box::new(iter),
                        min_last_bit_index(first.last_bit_index, second.last_bit_index),
                    )
                }
            },
            BitPageVecKind::AllOnes => first,
            BitPageVecKind::SparseWithOnesHole => match second.kind {
                BitPageVecKind::AllZeroes => first,
                BitPageVecKind::SparseWithZeroesHole => {
                    // merge here... cross type
                    let iter = second.iter.merge_join_by(first.iter, merge_cmp).filter_map(or_merge_cross_types);

                    // return type would be SparseWithOnesHole
                    BitPageVecIter::new(
                        BitPageVecKind::SparseWithOnesHole,
                        Box::new(iter),
                        min_last_bit_index(first.last_bit_index, second.last_bit_index),
                    )
                }
                BitPageVecKind::AllOnes => second,
                BitPageVecKind::SparseWithOnesHole => {
                    // merge here... same type with ones hole
                    // 1 | 1 => 1
                    // some | 1 => 1
                    // 1 | some => 1
                    // some | some => or(some)
                    // where 1 is hole
                    let iter = first.iter.merge_join_by(second.iter, merge_cmp).filter_map(|either| match either {
                        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
                            page_one |= page_two;

                            Some((idx_1, page_one))
                        }
                        EitherOrBoth::Left(_) | EitherOrBoth::Right(_) => None,
                    });

                    BitPageVecIter::new(
                        BitPageVecKind::SparseWithOnesHole,
                        Box::new(iter),
                        min_last_bit_index(first.last_bit_index, second.last_bit_index),
                    )
                }
            },
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "BitPageVecIter::OR result={:?}", result);
        }

        result
    }

    pub fn and(first: BitPageVecIter<'a>, second: BitPageVecIter<'a>) -> BitPageVecIter<'a> {
        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "BitPageVecIter::AND first={:?} second={:?}", first, second);
        }

        let result = match first.kind {
            BitPageVecKind::AllZeroes => first, // essentially AllZeroes
            BitPageVecKind::SparseWithZeroesHole => match second.kind {
                BitPageVecKind::AllZeroes => second,
                BitPageVecKind::SparseWithZeroesHole => {
                    // merge here... same type (with zeroes hole)
                    let iter = first.iter.merge_join_by(second.iter, merge_cmp).filter_map(|either| match either {
                        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
                            page_one &= page_two;

                            if BitPage::is_zeroes(&page_one) {
                                None
                            } else {
                                Some((idx_1, page_one))
                            }
                        }
                        EitherOrBoth::Left(_) | EitherOrBoth::Right(_) => None,
                    });

                    BitPageVecIter::new(
                        BitPageVecKind::SparseWithZeroesHole,
                        Box::new(iter),
                        min_last_bit_index(first.last_bit_index, second.last_bit_index),
                    )
                }
                BitPageVecKind::AllOnes => first,
                BitPageVecKind::SparseWithOnesHole => {
                    // merge here... cross type
                    let iter = first.iter.merge_join_by(second.iter, merge_cmp).filter_map(and_merge_cross_types);

                    // return type would be SparseWithZeroesHole
                    BitPageVecIter::new(
                        BitPageVecKind::SparseWithZeroesHole,
                        Box::new(iter),
                        min_last_bit_index(first.last_bit_index, second.last_bit_index),
                    )
                }
            },
            BitPageVecKind::AllOnes => second,
            BitPageVecKind::SparseWithOnesHole => match second.kind {
                BitPageVecKind::AllZeroes => second, // essentially AllZeroes
                BitPageVecKind::SparseWithZeroesHole => {
                    // merge here... cross type
                    // reverse the merge join... so first is always sparse with zeroes and second is always sparse with ones
                    let iter = second.iter.merge_join_by(first.iter, merge_cmp).filter_map(and_merge_cross_types);

                    // return type would be SparseWithZeroesHole
                    BitPageVecIter::new(
                        BitPageVecKind::SparseWithZeroesHole,
                        Box::new(iter),
                        min_last_bit_index(first.last_bit_index, second.last_bit_index),
                    )
                }
                BitPageVecKind::AllOnes => first,
                BitPageVecKind::SparseWithOnesHole => {
                    // merge here... same type (with ones hole)
                    let iter = first.iter.merge_join_by(second.iter, merge_cmp).map(|either| match either {
                        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
                            page_one &= page_two;

                            (idx_1, page_one)
                        }
                        EitherOrBoth::Left((idx, page)) | EitherOrBoth::Right((idx, page)) => (idx, page),
                    });
                    BitPageVecIter::new(
                        BitPageVecKind::SparseWithOnesHole,
                        Box::new(iter),
                        min_last_bit_index(first.last_bit_index, second.last_bit_index),
                    )
                }
            },
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "BitPageVecIter::AND result={:?}", result);
        }

        result
    }

    pub(crate) fn compact_sparse_with_zeroes_hole(pages: Vec<BitPageWithPosition>, last_bit_index: (usize, usize)) -> BitPageVec {
        if log_enabled!(target: "bit_page_vec_log", Level::Trace) {
            trace!(target: "bit_page_vec_log", "compact_sparse_with_zeroes_hole - pages len={}", pages.len());
        }

        let result = if pages.is_empty() {
            BitPageVec::all_zeros(last_bit_index)
        } else if pages.len() <= 10_000 {
            BitPageVec::new(BitPageVecKind::SparseWithZeroesHole, Some(pages), last_bit_index)
        } else {
            let start_page = pages[0].page_idx;
            let end_page = pages[pages.len() - 1].page_idx;
            let max_possible_length = (end_page - start_page + 1) as f64;
            let actual_length = pages.len() as f64;

            if log_enabled!(target: "bit_page_vec_log", Level::Trace) {
                trace!(target: "bit_page_vec_log", "compact_sparse_with_zeroes_hole - start_page={} end_page={} max_possible_length={} actual_length={}", start_page, end_page, max_possible_length, actual_length);
            }

            // find start page, end page, and length
            // if length >= 75% of (end - start) page
            // and # of active bits >= 75% of active bits needed for fully packed 75%
            if actual_length >= 0.75 * max_possible_length
                && BitPageVec::count_ones(Some(&pages)) as f64 >= 0.75 * max_possible_length * 64.0
            {
                if log_enabled!(target: "bit_page_vec_log", Level::Trace) {
                    trace!(target: "bit_page_vec_log", "compact_sparse_with_zeroes_hole::compacting - ones={}", BitPageVec::count_ones(Some(&pages)));
                }

                // filter out all page with max value
                // and include pages with holes
                let pages = (0..=last_bit_index.0)
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

                BitPageVec::new(BitPageVecKind::SparseWithOnesHole, Some(pages), last_bit_index)
            } else {
                BitPageVec::new(BitPageVecKind::SparseWithZeroesHole, Some(pages), last_bit_index)
            }
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Trace) {
            trace!(target: "bit_page_vec_log", "compact_sparse_with_zeroes_hole::result={:?}", result);
        }

        result
    }

    pub(crate) fn compact_sparse_with_ones_hole(pages: Vec<BitPageWithPosition>, last_bit_index: (usize, usize)) -> BitPageVec {
        if log_enabled!(target: "bit_page_vec_log", Level::Trace) {
            trace!(target: "bit_page_vec_log", "compact_sparse_with_ones_hole - pages len={}", pages.len());
        }

        let result = if pages.is_empty() {
            BitPageVec::all_ones(last_bit_index)
        } else if pages.len() <= 10_000 {
            BitPageVec::new(BitPageVecKind::SparseWithOnesHole, Some(pages), last_bit_index)
        } else {
            let start_page = pages[0].page_idx;
            let end_page = pages[pages.len() - 1].page_idx;
            let max_possible_length = (end_page - start_page + 1) as f64;
            let actual_length = pages.len() as f64;

            if log_enabled!(target: "bit_page_vec_log", Level::Trace) {
                debug!(target: "bit_page_vec_log", "compact_sparse_with_ones_hole - start_page={} end_page={} max_possible_length={} actual_length={}", start_page, end_page, max_possible_length, actual_length);
            }

            // find start page, end page, and length
            // if length >= 75% of (end - start) page
            // and # of active bits <= 25% of active bits needed for fully packed 75%
            if actual_length >= 0.75 * max_possible_length
                && BitPageVec::count_ones(Some(&pages)) as f64 <= 0.25 * max_possible_length * 64.0
            {
                if log_enabled!(target: "bit_page_vec_log", Level::Trace) {
                    debug!(target: "bit_page_vec_log", "compact_sparse_with_ones_hole::compacting - ones={}", BitPageVec::count_ones(Some(&pages)));
                }

                // filter out all page with max value
                // and include pages with holes
                let pages = (0..=last_bit_index.0)
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

                BitPageVec::new(BitPageVecKind::SparseWithZeroesHole, Some(pages), last_bit_index)
            } else {
                BitPageVec::new(BitPageVecKind::SparseWithOnesHole, Some(pages), last_bit_index)
            }
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Trace) {
            trace!(target: "bit_page_vec_log", "compact_sparse_with_ones_hole::result={:?}", result);
        }

        result
    }
}

impl BitPageVec {
    pub fn iter(&self) -> BitPageVecIter {
        match self.kind {
            BitPageVecKind::AllZeroes => {
                let iter = empty::<PageItem>();
                BitPageVecIter::new(BitPageVecKind::AllZeroes, Box::new(iter), self.last_bit_index)
            }
            BitPageVecKind::SparseWithZeroesHole => {
                if let Some(ref pages) = self.pages {
                    let iter = pages
                        .iter()
                        .map(|BitPageWithPosition { page_idx, bit_page }| (*page_idx, *bit_page));
                    BitPageVecIter::new(BitPageVecKind::SparseWithZeroesHole, Box::new(iter), self.last_bit_index)
                } else {
                    let iter = empty::<PageItem>();
                    BitPageVecIter::new(BitPageVecKind::AllZeroes, Box::new(iter), self.last_bit_index)
                }
            }
            BitPageVecKind::AllOnes => {
                let iter = empty::<PageItem>();
                BitPageVecIter::new(BitPageVecKind::AllOnes, Box::new(iter), self.last_bit_index)
            }
            BitPageVecKind::SparseWithOnesHole => {
                if let Some(ref pages) = self.pages {
                    let iter = pages
                        .iter()
                        .map(|BitPageWithPosition { page_idx, bit_page }| (*page_idx, *bit_page));
                    BitPageVecIter::new(BitPageVecKind::SparseWithOnesHole, Box::new(iter), self.last_bit_index)
                } else {
                    let iter = empty::<PageItem>();
                    BitPageVecIter::new(BitPageVecKind::AllOnes, Box::new(iter), self.last_bit_index)
                }
            }
        }
    }

    pub fn into_iter<'a>(self) -> BitPageVecIter<'a> {
        match self.kind {
            BitPageVecKind::AllZeroes => {
                let iter = empty::<PageItem>();
                BitPageVecIter::new(BitPageVecKind::AllZeroes, Box::new(iter), self.last_bit_index)
            }
            BitPageVecKind::SparseWithZeroesHole => {
                if let Some(pages) = self.pages {
                    let iter = pages
                        .into_iter()
                        .map(|BitPageWithPosition { page_idx, bit_page }| (page_idx, bit_page));
                    BitPageVecIter::new(BitPageVecKind::SparseWithZeroesHole, Box::new(iter), self.last_bit_index)
                } else {
                    let iter = empty::<PageItem>();
                    BitPageVecIter::new(BitPageVecKind::AllZeroes, Box::new(iter), self.last_bit_index)
                }
            }
            BitPageVecKind::AllOnes => {
                let iter = empty::<PageItem>();
                BitPageVecIter::new(BitPageVecKind::AllOnes, Box::new(iter), self.last_bit_index)
            }
            BitPageVecKind::SparseWithOnesHole => {
                if let Some(pages) = self.pages {
                    let iter = pages
                        .into_iter()
                        .map(|BitPageWithPosition { page_idx, bit_page }| (page_idx, bit_page));
                    BitPageVecIter::new(BitPageVecKind::SparseWithOnesHole, Box::new(iter), self.last_bit_index)
                } else {
                    let iter = empty::<PageItem>();
                    BitPageVecIter::new(BitPageVecKind::AllOnes, Box::new(iter), self.last_bit_index)
                }
            }
        }
    }
}

pub(crate) fn merge_cmp((idx_1, _): &PageItem, (idx_2, _): &PageItem) -> Ordering {
    idx_1.cmp(idx_2)
}

#[inline]
// first one is sparse with zeroes, second one is sparse with ones
// i.e. first hole = 0 and second hole = 1
// MISSING: first hole(0) | second hole(1) => second hole (1)... this is missing index... return type would be sparse with ones
// RIGHT: first hole(0) | some => some
// LEFT: some | second hole(1) => second hole(1)... should be filtered... return type would be sparse with ones
// BOTH: some | some => some
pub(crate) fn or_merge_cross_types(either: EitherOrBoth<PageItem, PageItem>) -> Option<PageItem> {
    match either {
        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
            page_one |= page_two;

            // some | some
            Some((idx_1, page_one))
        }
        EitherOrBoth::Left(_) => None, // some | 1
        EitherOrBoth::Right((idx, page)) => {
            // 0 | some
            Some((idx, page))
        }
    }
}

#[inline]
// first one is sparse with zeroes, second one is sparse with ones
// i.e. first hole = 0 and second hole = 1
// * MISSING: first hole & second hole => 0... return type would be sparse with zeroes
// * RIGHT: first hole & some => 0... should be filtered as return type would be sparse with zeroes
// * LEFT: some & second hole => some
// * BOTH: some & some => some
pub(crate) fn and_merge_cross_types(either: EitherOrBoth<PageItem, PageItem>) -> Option<PageItem> {
    match either {
        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
            page_one &= page_two;

            // some & some
            Some((idx_1, page_one))
        }
        EitherOrBoth::Left((idx, page)) => {
            // some & 1
            Some((idx, page))
        }
        EitherOrBoth::Right(_) => None, // 0 & some
    }
}

pub(crate) fn min_last_bit_index(first: (usize, usize), second: (usize, usize)) -> (usize, usize) {
    min(first, second)
}

#[cfg(test)]
mod tests {}
