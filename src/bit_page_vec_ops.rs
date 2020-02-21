use crate::{BitPageVec, DbBitPageVec};
// @author shailendra.sharma
use crate::bit_page_vec::BitPageVecKind;
use crate::bit_page_vec_iter::BitPageVecIter;

impl BitPageVec {
    pub fn or(&mut self, second: &BitPageVec) {
        let first = self.iter();
        let second = second.iter();

        *self = BitPageVecIter::or(first, second).into_bit_page_vec();
    }

    pub fn and(&mut self, second: &BitPageVec) {
        let first = self.iter();
        let second = second.iter();

        *self = BitPageVecIter::and(first, second).into_bit_page_vec();
    }

    pub fn not(&mut self) {
        *self = self.iter().not().into_bit_page_vec();
    }

    pub fn add(self, db_value: DbBitPageVec) -> BitPageVec {
        let bit_page_vec = match db_value {
            DbBitPageVec::AllZeroes => BitPageVec::all_zeros(self.last_bit_index),
            DbBitPageVec::Sparse(pages) => BitPageVec::new(BitPageVecKind::SparseWithZeroesHole, Some(pages), self.last_bit_index),
        };

        let first = self.into_iter();

        BitPageVecIter::or(first, bit_page_vec.into_iter()).into_bit_page_vec()
    }
}
