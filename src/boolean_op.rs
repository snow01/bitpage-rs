// @author shailendra.sharma
use itertools::Itertools;
use log::{debug, log_enabled, Level};

use crate::bit_page_vec_iter::BitPageVecIter;
use crate::BitPageVec;

#[derive(Clone, Debug)]
pub enum BooleanOp<'a> {
    And(Vec<BooleanOp<'a>>),
    Or(Vec<BooleanOp<'a>>),
    Not(Box<BooleanOp<'a>>),
    BorrowedLeaf(&'a BitPageVec),
    OwnedLeaf(BitPageVec),
}

#[derive(Debug)]
pub struct BooleanOpResult<'a> {
    len: usize,
    iter: BitPageVecIter<'a>,
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
        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "evaluate boolean_op={:?}", self);
        }

        let result = match self {
            BooleanOp::And(ops) => {
                // find max of start_page
                // find min of end_page
                // merge results
                let leaves = ops.into_iter().map(|op| op.evaluate(start_page, end_page)).collect_vec();

                Self::and_merge_leaves(leaves, start_page, end_page)
            }
            BooleanOp::Or(ops) => {
                // find min of start_page
                // find max of end_page
                // merge results
                let leaves = ops.into_iter().map(|op| op.evaluate(start_page, end_page)).collect_vec();

                Self::or_merge_leaves(leaves, start_page, end_page)
            }
            BooleanOp::Not(op) => op.evaluate(start_page, end_page).not(start_page, end_page),
            BooleanOp::BorrowedLeaf(leaf) => BooleanOpResult {
                len: leaf.size(),
                iter: leaf.iter(),
            },
            BooleanOp::OwnedLeaf(leaf) => BooleanOpResult {
                len: leaf.size(),
                iter: leaf.into_iter(),
            },
        };

        if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
            debug!(target: "bit_page_vec_log", "evaluate boolean_op result={:?}", result);
        }

        result
    }

    fn and_merge_leaves(mut leaves: Vec<BooleanOpResult<'a>>, start_page: usize, end_page: usize) -> BooleanOpResult<'a> {
        let mut merged_iter: Option<BitPageVecIter<'a>> = None;

        for leaf in leaves.drain(..) {
            match merged_iter {
                None => {
                    merged_iter = Some(leaf.iter);
                }
                Some(first) => {
                    merged_iter = Some(BitPageVecIter::and(first, leaf.iter));
                }
            }
        }

        BooleanOpResult {
            len: (end_page - start_page),
            iter: merged_iter.unwrap(),
        }
    }

    fn or_merge_leaves(mut leaves: Vec<BooleanOpResult<'a>>, start_page: usize, end_page: usize) -> BooleanOpResult<'a> {
        let mut merged_iter: Option<BitPageVecIter<'a>> = None;

        for leaf in leaves.drain(..) {
            match merged_iter {
                None => {
                    merged_iter = Some(leaf.iter);
                }
                Some(first) => {
                    merged_iter = Some(BitPageVecIter::or(first, leaf.iter));
                }
            }
        }

        BooleanOpResult {
            len: (end_page - start_page),
            iter: merged_iter.unwrap(),
        }
    }
}

impl<'a> BooleanOpResult<'a> {
    pub fn into_bit_page_vec(self) -> BitPageVec {
        self.iter.into_bit_page_vec()
    }

    // how to do this in fluent pattern... looks like it is hard in Rust (to google later)
    fn not(self, start_page: usize, end_page: usize) -> BooleanOpResult<'a> {
        let iter = self.iter.not();

        BooleanOpResult {
            len: (end_page - start_page),
            iter,
        }
    }
}
