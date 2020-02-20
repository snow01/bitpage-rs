use crate::bit_page_vec_iter::BitPageVecIter;
use crate::BitPageVec;

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
}
