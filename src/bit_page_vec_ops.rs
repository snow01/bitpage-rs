use std::borrow::Cow;
use std::cmp::{max, min, Ordering};
use std::iter::empty;

use itertools::{EitherOrBoth, Itertools};

use crate::bit_page_vec::BitPageWithPosition;
use crate::{BitPage, BitPageVec};

#[derive(Clone, Debug)]
pub enum BooleanOp<'a> {
    And(Vec<BooleanOp<'a>>),
    Or(Vec<BooleanOp<'a>>),
    Not(Box<BooleanOp<'a>>),
    BorrowedLeaf(&'a BitPageVec),
    OwnedLeaf(BitPageVec),
}

pub struct BooleanOpResult<'a> {
    start_page: usize,
    end_page: usize,
    len: usize,
    iter: PageIterator<'a>,
}

impl<'a> BooleanOp<'a> {
    pub fn new_leaf_op(bit_page_vec: &'a BitPageVec) -> BooleanOp<'a> {
        BooleanOp::BorrowedLeaf(bit_page_vec)
    }

    pub fn new_owned_leaf_op(bit_page_vec: BitPageVec) -> BooleanOp<'a> {
        BooleanOp::OwnedLeaf(bit_page_vec)
    }

    pub fn new_and_op(mut ops: Vec<BooleanOp<'a>>) -> anyhow::Result<BooleanOp<'a>> {
        anyhow::ensure!(!ops.is_empty(), "For 'and' op minimum one sub op should be there");

        if ops.len() == 1 {
            // simplify AND with single op
            Ok(ops.pop().unwrap())
        } else {
            Ok(BooleanOp::And(ops))
        }
    }

    pub fn new_or_op(mut ops: Vec<BooleanOp<'a>>) -> anyhow::Result<BooleanOp<'a>> {
        anyhow::ensure!(!ops.is_empty(), "For 'or' op minimum one sub op should be there");

        if ops.len() == 1 {
            // simplify OR with single op
            Ok(ops.pop().unwrap())
        } else {
            Ok(BooleanOp::Or(ops))
        }
    }

    pub fn new_not_op(op: BooleanOp<'a>) -> BooleanOp<'a> {
        BooleanOp::Not(Box::new(op))
    }

    pub fn evaluate(self, start_page: usize, end_page: usize) -> BooleanOpResult<'a> {
        match self {
            BooleanOp::And(ops) => {
                // find max of start_page
                // find min of end_page
                let mut start_page_inner = start_page;
                let mut end_page_inner = end_page;

                let mut leaves = Vec::with_capacity(ops.len());

                for op in ops {
                    let leaf = op.evaluate(start_page_inner, end_page_inner);
                    start_page_inner = max(start_page_inner, leaf.start_page);
                    end_page_inner = min(start_page_inner, leaf.end_page);

                    if leaf.len == 0 {
                        // short-circuit here
                        return BooleanOpResult {
                            start_page: 0,
                            end_page: 0,
                            len: 0,
                            iter: Box::new(empty()),
                        };
                    }

                    leaves.push(leaf);
                }

                // merge results
                if leaves.len() > 1 {
                    leaves.sort_unstable_by(|leaf_1, leaf_2| leaf_1.len.cmp(&leaf_2.len));
                }

                Self::and_merge_leaves(leaves, start_page_inner, end_page_inner)
            }
            BooleanOp::Or(ops) => {
                // find min of start_page
                // find max of end_page
                let mut start_page_inner = start_page;
                let mut end_page_inner = end_page;

                // merge results
                let leaves = ops
                    .into_iter()
                    .map(|op| {
                        let leaf = op.evaluate(start_page, end_page);
                        start_page_inner = min(start_page_inner, leaf.start_page);
                        end_page_inner = max(start_page_inner, leaf.end_page);
                        leaf
                    })
                    .collect_vec();

                Self::or_merge_leaves(leaves, start_page_inner, end_page_inner)
            }
            BooleanOp::Not(op) => op.evaluate(start_page, end_page).not(start_page, end_page),
            BooleanOp::BorrowedLeaf(leaf) => {
                // TODO: limit iter as per start_page and end_page parameter
                let start_page = leaf.start_page();
                let end_page = leaf.end_page();
                BooleanOpResult {
                    start_page,
                    end_page,
                    len: leaf.len(),
                    iter: leaf.iter(),
                }
            }
            BooleanOp::OwnedLeaf(leaf) => {
                // TODO: limit iter as per start_page and end_page parameter
                let start_page = leaf.start_page();
                let end_page = leaf.end_page();
                BooleanOpResult {
                    start_page,
                    end_page,
                    len: leaf.len(),
                    iter: leaf.into_iter(),
                }
            }
        }
    }

    fn and_merge_leaves(mut leaves: Vec<BooleanOpResult<'a>>, start_page: usize, end_page: usize) -> BooleanOpResult<'a> {
        let mut iter: Option<PageIterator<'a>> = None;

        for leaf in leaves.drain(..) {
            match iter {
                None => {
                    iter = Some(leaf.iter);
                }
                Some(some) => {
                    let iter_inner = some.merge_join_by(leaf.iter, merge_cmp).filter_map(and_merge_iter);

                    iter = Some(Box::new(iter_inner));
                }
            }
        }

        BooleanOpResult {
            start_page,
            end_page,
            len: (end_page - start_page),
            iter: iter.unwrap(),
        }
    }

    fn or_merge_leaves(mut leaves: Vec<BooleanOpResult<'a>>, start_page: usize, end_page: usize) -> BooleanOpResult<'a> {
        let mut iter: Option<PageIterator<'a>> = None;

        for leaf in leaves.drain(..) {
            match iter {
                None => {
                    iter = Some(leaf.iter);
                }
                Some(some) => {
                    let iter_inner = some.merge_join_by(leaf.iter, merge_cmp).map(or_merge_iter);

                    iter = Some(Box::new(iter_inner));
                }
            }
        }

        BooleanOpResult {
            start_page,
            end_page,
            len: (end_page - start_page),
            iter: iter.unwrap(),
        }
    }
}

impl<'a> BooleanOpResult<'a> {
    pub fn convert_to_bit_page_vec(self) -> BitPageVec {
        let pages = self
            .iter
            .filter_map(|(page_idx, bit_page)| {
                if bit_page.is_all_zeros() {
                    None
                } else {
                    Some(BitPageWithPosition {
                        page_idx: page_idx.into_owned(),
                        bit_page: bit_page.into_owned(),
                    })
                }
            })
            .collect_vec();

        if pages.is_empty() {
            BitPageVec::AllZeroes
        } else {
            BitPageVec::Sparse(pages)
        }
    }

    fn not(self, start_page: usize, end_page: usize) -> BooleanOpResult<'a> {
        let iter = (start_page..end_page)
            .merge_join_by(self.iter, |page_idx_1, (page_idx_2, _)| page_idx_1.cmp(page_idx_2))
            .filter_map(|either| match either {
                EitherOrBoth::Left(page) => {
                    // create a new page with ones and return...
                    Some((Cow::Owned(page), Cow::Owned(BitPage::ones())))
                }
                EitherOrBoth::Right(_) => {
                    // should be ignored
                    None
                }
                EitherOrBoth::Both(_, (page_idx_2, mut bit_page)) => {
                    bit_page.to_mut().not();

                    if BitPage::Zeroes.eq(&bit_page) {
                        None
                    } else {
                        Some((page_idx_2, bit_page))
                    }
                }
            });

        BooleanOpResult {
            start_page,
            end_page,
            len: (end_page - start_page),
            iter: Box::new(iter),
        }
    }
}

pub type PageItem<'a> = (Cow<'a, usize>, Cow<'a, BitPage>);
pub type PageIterator<'a> = Box<dyn Iterator<Item = PageItem<'a>> + 'a>;

impl BitPageVec {
    fn iter<'a>(&'a self) -> PageIterator<'a> {
        match self {
            BitPageVec::AllZeroes => {
                let iter = empty::<PageItem<'a>>();
                Box::new(iter)
            }
            BitPageVec::Sparse(pages) => {
                let iter = pages
                    .iter()
                    .map(|BitPageWithPosition { page_idx, bit_page }| (Cow::Borrowed(page_idx), Cow::Borrowed(bit_page)));
                Box::new(iter)
            }
        }
    }

    fn into_iter<'a>(self) -> PageIterator<'a> {
        match self {
            BitPageVec::AllZeroes => {
                let iter = empty::<PageItem<'a>>();
                Box::new(iter)
            }
            BitPageVec::Sparse(pages) => {
                let iter = pages
                    .into_iter()
                    .map(|BitPageWithPosition { page_idx, bit_page }| (Cow::Owned(page_idx), Cow::Owned(bit_page)));
                Box::new(iter)
            }
        }
    }

    pub fn or(&mut self, second: &BitPageVec) {
        let pages = self
            .iter()
            .merge_join_by(second.iter(), merge_cmp)
            .map(or_merge_iter)
            .map(|(page_idx, bit_page)| BitPageWithPosition {
                page_idx: page_idx.into_owned(),
                bit_page: bit_page.into_owned(),
            })
            .collect_vec();

        *self = BitPageVec::Sparse(pages)
    }

    pub fn and(&mut self, second: &BitPageVec) {
        let pages = self
            .iter()
            .merge_join_by(second.iter(), merge_cmp)
            .filter_map(and_merge_iter)
            .map(|(page_idx, bit_page)| BitPageWithPosition {
                page_idx: page_idx.into_owned(),
                bit_page: bit_page.into_owned(),
            })
            .collect_vec();

        if pages.is_empty() {
            *self = BitPageVec::all_zeros()
        } else {
            *self = BitPageVec::Sparse(pages)
        }
    }

    pub fn not(&mut self, num_pages: usize) {
        let pages = (0..=num_pages)
            .merge_join_by(self.iter(), |idx_1, (idx_2, _)| idx_1.cmp(idx_2))
            .filter_map(|either| match either {
                EitherOrBoth::Both(_, (page_idx, mut bit_page)) => {
                    bit_page.to_mut().not();
                    if BitPage::Zeroes.eq(&bit_page) {
                        None
                    } else {
                        Some((page_idx, bit_page))
                    }
                }
                EitherOrBoth::Left(page_idx) => Some((Cow::Owned(page_idx), Cow::Owned(BitPage::Ones))),
                EitherOrBoth::Right(_) => None,
            })
            .map(|(page_idx, bit_page)| BitPageWithPosition {
                page_idx: page_idx.into_owned(),
                bit_page: bit_page.into_owned(),
            })
            .collect_vec();

        if pages.is_empty() {
            *self = BitPageVec::all_zeros()
        } else {
            *self = BitPageVec::Sparse(pages)
        }
    }
}

fn merge_cmp((idx_1, _): &PageItem<'_>, (idx_2, _): &PageItem<'_>) -> Ordering {
    idx_1.cmp(idx_2)
}

#[inline]
fn and_merge_iter<'a>(either: EitherOrBoth<PageItem<'a>, PageItem<'a>>) -> Option<PageItem<'a>> {
    match either {
        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
            page_one.to_mut().and(&page_two);

            if BitPage::Zeroes.eq(&page_one) {
                None
            } else {
                Some((idx_1, page_one))
            }
        }
        EitherOrBoth::Left(_) | EitherOrBoth::Right(_) => None,
    }
}

#[inline]
fn or_merge_iter<'a>(either: EitherOrBoth<PageItem<'a>, PageItem<'a>>) -> PageItem<'a> {
    match either {
        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
            page_one.to_mut().or(&page_two);
            (idx_1, page_one)
        }
        EitherOrBoth::Left((idx, page)) => (idx, page),
        EitherOrBoth::Right((idx, page)) => (idx, page),
    }
}
