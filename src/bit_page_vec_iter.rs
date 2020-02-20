// @author shailendra.sharma
use std::cmp::Ordering;
use std::fmt;
use std::iter::empty;
use std::time::Instant;

use itertools::{EitherOrBoth, Itertools};
use log::{debug, log_enabled, Level};

use crate::bit_page_vec::BitPageWithPosition;
use crate::{BitPage, BitPageVec};

pub type PageItem = (usize, u64);
pub type PageIterator<'a> = Box<dyn Iterator<Item = PageItem> + 'a>;

#[derive(Copy, Clone, Debug)]
pub enum IterKind {
    AllZeroes,
    SparseWithZeroesHole,
    AllOnes,
    SparseWithOnesHole,
}

pub struct BitPageVecIter<'a> {
    kind: IterKind,
    iter: PageIterator<'a>,
}

impl<'a> fmt::Debug for BitPageVecIter<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BitPageVecIter(kind={:?})", self.kind)
    }
}

impl<'a> BitPageVecIter<'a> {
    pub fn new(kind: IterKind, iter: PageIterator) -> BitPageVecIter {
        BitPageVecIter { kind, iter }
    }

    pub fn kind(&self) -> &IterKind {
        &self.kind
    }

    pub fn into_bit_page_vec(self) -> BitPageVec {
        let instant = Instant::now();
        let kind = self.kind;

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "into_bit_page_vec(kind={:?})", self.kind);
        }

        let result = match self.kind {
            IterKind::AllZeroes => BitPageVec::AllZeroes,
            IterKind::SparseWithZeroesHole => {
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

                BitPageVec::compact_sparse_with_zeroes_hole(pages)
            }
            IterKind::AllOnes => BitPageVec::AllOnes,
            IterKind::SparseWithOnesHole => {
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

                BitPageVec::compact_sparse_with_ones_hole(pages)
            }
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "into_bit_page_vec(kind={:?}):: time taken={:?} and result={:?}", kind, instant.elapsed(), result);
        }

        result
    }

    pub fn not(mut self) -> BitPageVecIter<'a> {
        match self.kind {
            IterKind::AllZeroes => {
                self.kind = IterKind::AllOnes;
            }
            IterKind::SparseWithZeroesHole => {
                self.kind = IterKind::SparseWithOnesHole;
                self.iter = Box::new(self.iter.map(|(page_idx, bit_page)| (page_idx, !bit_page)));
            }
            IterKind::AllOnes => {
                self.kind = IterKind::AllZeroes;
            }
            IterKind::SparseWithOnesHole => {
                self.kind = IterKind::SparseWithZeroesHole;
                self.iter = Box::new(self.iter.map(|(page_idx, bit_page)| (page_idx, !bit_page)));
            }
        }

        self
    }

    pub fn or(first: BitPageVecIter<'a>, second: BitPageVecIter<'a>) -> BitPageVecIter<'a> {
        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "BitPageVecIter::OR first={:?} second={:?}", first, second);
        }

        let result = match first.kind {
            IterKind::AllZeroes => second,
            IterKind::SparseWithZeroesHole => match second.kind {
                IterKind::AllZeroes => first,
                IterKind::SparseWithZeroesHole => {
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

                    BitPageVecIter::new(IterKind::SparseWithZeroesHole, Box::new(iter))
                }
                IterKind::AllOnes => second,
                IterKind::SparseWithOnesHole => {
                    // merge here... cross type
                    let iter = first.iter.merge_join_by(second.iter, merge_cmp).filter_map(or_merge_cross_types);
                    BitPageVecIter::new(IterKind::SparseWithOnesHole, Box::new(iter))
                }
            },
            IterKind::AllOnes => first,
            IterKind::SparseWithOnesHole => match second.kind {
                IterKind::AllZeroes => first,
                IterKind::SparseWithZeroesHole => {
                    // merge here... cross type
                    let iter = second.iter.merge_join_by(first.iter, merge_cmp).filter_map(or_merge_cross_types);

                    // return type would be SparseWithOnesHole
                    BitPageVecIter::new(IterKind::SparseWithOnesHole, Box::new(iter))
                }
                IterKind::AllOnes => second,
                IterKind::SparseWithOnesHole => {
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

                    BitPageVecIter::new(IterKind::SparseWithOnesHole, Box::new(iter))
                }
            },
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "BitPageVecIter::AND result={:?}", result);
        }

        result
    }

    pub fn and(first: BitPageVecIter<'a>, second: BitPageVecIter<'a>) -> BitPageVecIter<'a> {
        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "BitPageVecIter::AND first={:?} second={:?}", first, second);
        }

        let result = match first.kind {
            IterKind::AllZeroes => first, // essentially AllZeroes
            IterKind::SparseWithZeroesHole => match second.kind {
                IterKind::AllZeroes => second,
                IterKind::SparseWithZeroesHole => {
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

                    BitPageVecIter::new(IterKind::SparseWithZeroesHole, Box::new(iter))
                }
                IterKind::AllOnes => first,
                IterKind::SparseWithOnesHole => {
                    // merge here... cross type
                    let iter = first.iter.merge_join_by(second.iter, merge_cmp).filter_map(and_merge_cross_types);

                    // return type would be SparseWithZeroesHole
                    BitPageVecIter::new(IterKind::SparseWithZeroesHole, Box::new(iter))
                }
            },
            IterKind::AllOnes => second,
            IterKind::SparseWithOnesHole => match second.kind {
                IterKind::AllZeroes => second, // essentially AllZeroes
                IterKind::SparseWithZeroesHole => {
                    // merge here... cross type
                    // reverse the merge join... so first is always sparse with zeroes and second is always sparse with ones
                    let iter = second.iter.merge_join_by(first.iter, merge_cmp).filter_map(and_merge_cross_types);

                    // return type would be SparseWithZeroesHole
                    BitPageVecIter::new(IterKind::SparseWithZeroesHole, Box::new(iter))
                }
                IterKind::AllOnes => first,
                IterKind::SparseWithOnesHole => {
                    // merge here... same type (with ones hole)
                    let iter = first.iter.merge_join_by(second.iter, merge_cmp).map(|either| match either {
                        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
                            page_one &= page_two;

                            (idx_1, page_one)
                        }
                        EitherOrBoth::Left((idx, page)) | EitherOrBoth::Right((idx, page)) => (idx, page),
                    });
                    BitPageVecIter::new(IterKind::SparseWithOnesHole, Box::new(iter))
                }
            },
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "BitPageVecIter::AND result={:?}", result);
        }

        result
    }
}

impl BitPageVec {
    pub fn iter(&self) -> BitPageVecIter {
        match self {
            BitPageVec::AllZeroes => {
                let iter = empty::<PageItem>();

                BitPageVecIter::new(IterKind::AllZeroes, Box::new(iter))
            }
            BitPageVec::SparseWithZeroesHole(pages) => {
                let iter = pages
                    .iter()
                    .map(|BitPageWithPosition { page_idx, bit_page }| (*page_idx, *bit_page));
                BitPageVecIter::new(IterKind::SparseWithZeroesHole, Box::new(iter))
            }
            BitPageVec::AllOnes => {
                let iter = empty::<PageItem>();
                BitPageVecIter::new(IterKind::AllOnes, Box::new(iter))
            }
            BitPageVec::SparseWithOnesHole(pages) => {
                let iter = pages
                    .iter()
                    .map(|BitPageWithPosition { page_idx, bit_page }| (*page_idx, *bit_page));
                BitPageVecIter::new(IterKind::SparseWithOnesHole, Box::new(iter))
            }
        }
    }

    pub fn into_iter<'a>(self) -> BitPageVecIter<'a> {
        match self {
            BitPageVec::AllZeroes => {
                let iter = empty::<PageItem>();
                BitPageVecIter::new(IterKind::AllZeroes, Box::new(iter))
            }
            BitPageVec::SparseWithZeroesHole(pages) => {
                let iter = pages
                    .into_iter()
                    .map(|BitPageWithPosition { page_idx, bit_page }| (page_idx, bit_page));
                BitPageVecIter::new(IterKind::SparseWithZeroesHole, Box::new(iter))
            }
            BitPageVec::AllOnes => {
                let iter = empty::<PageItem>();
                BitPageVecIter::new(IterKind::AllOnes, Box::new(iter))
            }
            BitPageVec::SparseWithOnesHole(pages) => {
                let iter = pages
                    .into_iter()
                    .map(|BitPageWithPosition { page_idx, bit_page }| (page_idx, bit_page));
                BitPageVecIter::new(IterKind::SparseWithOnesHole, Box::new(iter))
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
