use std::cmp::{max, min};

use crate::bit_page_vec_iter::BitPageVecIter;
use crate::BitPageVec;

// @author shailendra.sharma

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

    pub fn evaluate(self) -> BooleanOpResult<'a> {
        // if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
        //     debug!(target: "bit_page_vec_log", "evaluate boolean_op={:?}", self);
        // }

        let result = match self {
            BooleanOp::And(ops) => ops.into_iter().map(|op| op.evaluate()).and_merge_leaves(),
            BooleanOp::Or(ops) => ops.into_iter().map(|op| op.evaluate()).or_merge_leaves(),
            BooleanOp::Not(op) => op.evaluate().not(),
            BooleanOp::BorrowedLeaf(leaf) => BooleanOpResult {
                len: leaf.size(),
                iter: leaf.iter(),
            },
            BooleanOp::OwnedLeaf(leaf) => BooleanOpResult {
                len: leaf.size(),
                iter: leaf.into_iter(),
            },
        };

        // if log_enabled!(target: "bit_page_vec_log", Level::Debug) {
        //     debug!(target: "bit_page_vec_log", "evaluate boolean_op result={:?}", result);
        // }

        result
    }

    // fn and_merge_leaves<I>(mut leaves: Vec<BooleanOpResult<'a>>) -> BooleanOpResult<'a>
    // where
    //     I: Iterator<Item = BooleanOp<'a>>,
    // {
    //     let mut merged_iter: Option<BitPageVecIter<'a>> = None;
    //
    //     let mut len = usize::max_value();
    //     for leaf in leaves {
    //         len = min(len, leaf.len);
    //         match merged_iter {
    //             None => merged_iter = Some(leaf.iter),
    //             Some(first) => merged_iter = Some(BitPageVecIter::and(first, leaf.iter)),
    //         }
    //     }
    //
    //     BooleanOpResult {
    //         len,
    //         iter: merged_iter.unwrap(),
    //     }
    // }

    // fn or_merge_leaves(mut leaves: Vec<BooleanOpResult<'a>>) -> BooleanOpResult<'a> {
    //     let mut merged_iter: Option<BitPageVecIter<'a>> = None;
    //
    //     let mut len = 0;
    //     for leaf in leaves.drain(..) {
    //         len = max(len, leaf.len);
    //         match merged_iter {
    //             None => merged_iter = { Some(leaf.iter) },
    //             Some(first) => merged_iter = Some(BitPageVecIter::or(first, leaf.iter)),
    //         }
    //     }
    //
    //     BooleanOpResult {
    //         len,
    //         iter: merged_iter.unwrap(),
    //     }
    // }
}

pub trait MergeLeavesIterator<'a>: Iterator<Item = BooleanOpResult<'a>> {
    fn and_merge_leaves(self) -> BooleanOpResult<'a>
    where
        Self: Sized,
    {
        let mut merged_iter: Option<BitPageVecIter<'a>> = None;

        let mut len = usize::max_value();
        for leaf in self.into_iter() {
            len = min(len, leaf.len);
            match merged_iter {
                None => merged_iter = Some(leaf.iter),
                Some(first) => merged_iter = Some(BitPageVecIter::and(first, leaf.iter)),
            }
        }

        BooleanOpResult {
            len,
            iter: merged_iter.unwrap(),
        }
    }

    fn or_merge_leaves(self) -> BooleanOpResult<'a>
    where
        Self: Sized,
    {
        let mut merged_iter: Option<BitPageVecIter<'a>> = None;

        let mut len = 0;
        for leaf in self.into_iter() {
            len = max(len, leaf.len);
            match merged_iter {
                None => merged_iter = { Some(leaf.iter) },
                Some(first) => merged_iter = Some(BitPageVecIter::or(first, leaf.iter)),
            }
        }

        BooleanOpResult {
            len,
            iter: merged_iter.unwrap(),
        }
    }
}

impl<'a, T: ?Sized> MergeLeavesIterator<'a> for T where T: Iterator<Item = BooleanOpResult<'a>> {}

impl<'a> BooleanOpResult<'a> {
    pub fn into_bit_page_vec(self) -> BitPageVec {
        self.iter.into_bit_page_vec()
    }

    // how to do this in fluent pattern... looks like it is hard in Rust (to google later)
    fn not(self) -> BooleanOpResult<'a> {
        let iter = self.iter.not();

        BooleanOpResult { len: self.len, iter }
    }
}
