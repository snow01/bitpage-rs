use crate::bit_page_vec::BitPageWithPosition;
use crate::{BitPage, BitPageVec};

impl BitPageVec {
    pub fn active_bits(&self) -> BitPageVecIterator {
        match self {
            BitPageVec::AllZeroes => BitPageVecIterator::None,
            BitPageVec::Sparse(pages) => {
                let iter = pages.iter().flat_map(|BitPageWithPosition { page_idx, bit_page }| {
                    BitPage::active_bits(bit_page).map(move |bit_idx| (*page_idx, bit_idx))
                });

                BitPageVecIterator::Some { iter: Box::new(iter) }
            }
        }
    }
}

pub enum BitPageVecIterator<'a> {
    None,
    Some {
        iter: Box<dyn Iterator<Item = (usize, usize)> + 'a>,
    },
}

impl<'a> Iterator for BitPageVecIterator<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            BitPageVecIterator::None => None,
            BitPageVecIterator::Some { iter } => iter.next(),
        }
    }
}
