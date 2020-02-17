//use log::{debug, log_enabled, Level};
use std::cmp::{max, min, Ordering};
use std::iter::empty;

use itertools::{EitherOrBoth, Itertools};

use crate::bit_page_vec::BitPageWithPosition;
use crate::{BitPage, BitPageVec};

pub enum BooleanOp<'a> {
    And(Vec<BooleanOp<'a>>),
    Or(Vec<BooleanOp<'a>>),
    Not(Box<BooleanOp<'a>>),
    Leaf(BooleanOpLeaf<'a>),
}

pub struct BooleanOpLeaf<'a> {
    start_page: usize,
    end_page: usize,
    iter: PageIterator<'a>,
}

impl<'a> BooleanOp<'a> {
    pub fn new_leaf_op(bitpage_vec: &'a BitPageVec) -> BooleanOp<'a> {
        let start_page = bitpage_vec.start_page();
        let end_page = bitpage_vec.end_page();
        let iter = bitpage_vec.iter();

        BooleanOp::Leaf(BooleanOpLeaf {
            start_page,
            end_page,
            iter,
        })
    }

    pub fn new_owned_leaf_op(bitpage_vec: BitPageVec) -> BooleanOp<'a> {
        let start_page = bitpage_vec.start_page();
        let end_page = bitpage_vec.end_page();
        let iter = bitpage_vec.into_iter();

        BooleanOp::Leaf(BooleanOpLeaf {
            start_page,
            end_page,
            iter,
        })
    }

    pub fn new_and_op(ops: Vec<BooleanOp<'a>>) -> anyhow::Result<BooleanOp<'a>> {
        anyhow::ensure!(!ops.is_empty(), "For 'and' op minimum one sub op should be there");

        // TODO: optimise if only one op

        Ok(BooleanOp::And(ops))
    }

    pub fn new_or_op(ops: Vec<BooleanOp<'a>>) -> anyhow::Result<BooleanOp<'a>> {
        anyhow::ensure!(!ops.is_empty(), "For 'or' op minimum one sub op should be there");

        // TODO: optimise if only one op

        Ok(BooleanOp::Or(ops))
    }

    pub fn new_not_op(op: BooleanOp<'a>) -> BooleanOp<'a> {
        BooleanOp::Not(Box::new(op))
    }

    pub fn evaluate(self, start_page: usize, end_page: usize) -> BooleanOpLeaf<'a> {
        match self {
            BooleanOp::And(ops) => {
                // find max of start_page
                // find min of end_page
                let mut start_page_inner = max(usize::min_value(), start_page);
                let mut end_page_inner = min(usize::max_value(), end_page);

                // merge results
                let leaves = ops
                    .into_iter()
                    .map(|op| {
                        let leaf = op.evaluate(start_page, end_page);
                        start_page_inner = max(start_page_inner, leaf.start_page);
                        end_page_inner = min(start_page_inner, leaf.end_page);
                        leaf
                    })
                    .collect_vec();

                Self::and_merge_leaves(leaves, start_page_inner, end_page_inner)
            }
            BooleanOp::Or(ops) => {
                // find min of start_page
                // find max of end_page

                let mut start_page_inner = min(usize::max_value(), start_page);
                let mut end_page_inner = max(usize::min_value(), end_page);

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
            BooleanOp::Leaf(leaf) => leaf,
        }
    }

    fn and_merge_leaves(mut leaves: Vec<BooleanOpLeaf<'a>>, start_page: usize, end_page: usize) -> BooleanOpLeaf<'a> {
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

        BooleanOpLeaf {
            start_page,
            end_page,
            iter: iter.unwrap(),
        }
    }

    fn or_merge_leaves(mut leaves: Vec<BooleanOpLeaf<'a>>, start_page: usize, end_page: usize) -> BooleanOpLeaf<'a> {
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

        BooleanOpLeaf {
            start_page,
            end_page,
            iter: iter.unwrap(),
        }
    }
}

impl<'a> BooleanOpLeaf<'a> {
    pub fn convert_to_bit_page_vec(self) -> BitPageVec {
        let pages = self
            .iter
            .filter_map(|(page_idx, bit_page)| {
                if bit_page.is_all_zeros() {
                    None
                } else {
                    Some(BitPageWithPosition { page_idx, bit_page })
                }
            })
            .collect_vec();

        if pages.is_empty() {
            BitPageVec::AllZeroes
        } else {
            BitPageVec::Sparse(pages)
        }
    }

    fn not(self, start_page: usize, end_page: usize) -> BooleanOpLeaf<'a> {
        let iter = (start_page..end_page)
            .merge_join_by(self.iter, |page_idx_1, (page_idx_2, _)| page_idx_1.cmp(page_idx_2))
            .filter_map(|either| match either {
                EitherOrBoth::Left(page) => {
                    // create a new page with ones and return...
                    Some((page, BitPage::ones()))
                }
                EitherOrBoth::Right(_) => {
                    // should be ignored
                    None
                }
                EitherOrBoth::Both(_, (page_idx_2, mut bit_page)) => {
                    bit_page.not();
                    Some((page_idx_2, bit_page))
                }
            });

        BooleanOpLeaf {
            start_page,
            end_page,
            iter: Box::new(iter),
        }
    }
}

pub type PageIterator<'a> = Box<dyn Iterator<Item = (usize, BitPage)> + 'a>;

impl BitPageVec {
    fn iter(&self) -> PageIterator {
        match self {
            BitPageVec::AllZeroes => {
                let iter = empty::<(usize, BitPage)>();
                Box::new(iter)
            }
            BitPageVec::Sparse(pages) => {
                let iter = pages
                    .iter()
                    .map(|BitPageWithPosition { page_idx, bit_page }| (*page_idx, bit_page.clone()));
                Box::new(iter)
            }
        }
    }

    fn into_iter<'a>(self) -> PageIterator<'a> {
        match self {
            BitPageVec::AllZeroes => {
                let iter = empty::<(usize, BitPage)>();
                Box::new(iter)
            }
            BitPageVec::Sparse(pages) => {
                let iter = pages
                    .into_iter()
                    .map(|BitPageWithPosition { page_idx, bit_page }| (page_idx, bit_page));
                Box::new(iter)
            }
        }
    }

    pub fn or(&mut self, second: &BitPageVec) {
        // TODO: do case basis
        let pages = self
            .iter()
            .merge_join_by(second.iter(), merge_cmp)
            .map(or_merge_iter)
            .map(|(page_idx, bit_page)| BitPageWithPosition { page_idx, bit_page })
            .collect_vec();

        *self = BitPageVec::Sparse(pages)
    }
}

fn merge_cmp((idx_1, _): &(usize, BitPage), (idx_2, _): &(usize, BitPage)) -> Ordering {
    idx_1.cmp(idx_2)
}

#[inline]
fn and_merge_iter<'a>(either: EitherOrBoth<(usize, BitPage), (usize, BitPage)>) -> Option<(usize, BitPage)> {
    match either {
        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
            page_one.and(&page_two);
            Some((idx_1, page_one))
        }
        EitherOrBoth::Left(_) | EitherOrBoth::Right(_) => None,
    }
}

#[inline]
fn or_merge_iter<'a>(either: EitherOrBoth<(usize, BitPage), (usize, BitPage)>) -> (usize, BitPage) {
    match either {
        EitherOrBoth::Both((idx_1, mut page_one), (_idx_2, page_two)) => {
            page_one.or(&page_two);
            (idx_1, page_one)
        }
        EitherOrBoth::Left((idx, page)) => (idx, page),
        EitherOrBoth::Right((idx, page)) => (idx, page),
    }
}
