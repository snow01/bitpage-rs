use std::fmt;

use crate::{BitPage, BitPageVec};
// @author shailendra.sharma
use crate::bit_page::BitPageWithPosition;

#[derive(Clone)]
pub enum DbBitPageVec {
    AllZeroes,
    Sparse(Vec<BitPageWithPosition>),
}

impl Default for DbBitPageVec {
    fn default() -> Self {
        DbBitPageVec::all_zeros()
    }
}

impl DbBitPageVec {
    #[inline]
    pub fn all_zeros() -> DbBitPageVec {
        DbBitPageVec::AllZeroes
    }

    #[inline]
    pub fn clear_bit(&mut self, page_idx: usize, bit_idx: usize) {
        match self {
            DbBitPageVec::AllZeroes => {
                // no-op
            }
            DbBitPageVec::Sparse(pages) => {
                // do binary search for page_idx...
                if let Ok(matching_index) = pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                    // clear bit at the matching index
                    let bit_page = &mut pages[matching_index].bit_page;
                    BitPage::clear_bit(bit_page, bit_idx);

                    if BitPage::is_zeroes(bit_page) {
                        // remove this bit page from matching index and compact page
                        pages.remove(matching_index);
                    }

                    // compact BitPageVec
                    if pages.is_empty() {
                        *self = DbBitPageVec::all_zeros();
                    }
                }
            }
        }
    }

    #[inline]
    pub fn set_bit(&mut self, page_idx: usize, bit_idx: usize) {
        match self {
            DbBitPageVec::AllZeroes => {
                let mut bit_page = BitPage::zeroes();
                BitPage::set_bit(&mut bit_page, bit_idx);

                *self = DbBitPageVec::Sparse(vec![BitPageWithPosition { page_idx, bit_page }]);
            }
            DbBitPageVec::Sparse(pages) => {
                // do binary search for page_idx...
                match pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                    Ok(matching_index) => {
                        // set bit at the matching index
                        let bit_page = &mut pages[matching_index].bit_page;
                        BitPage::set_bit(bit_page, bit_idx);
                    }
                    Err(insertion_index) => {
                        // create new page and insert at matching index
                        let mut bit_page = BitPage::zeroes();
                        BitPage::set_bit(&mut bit_page, bit_idx);

                        pages.insert(insertion_index, BitPageWithPosition { page_idx, bit_page });
                    }
                }
            }
        }
    }

    #[inline]
    pub fn get_bit_page_in_binary_format(&self, page_idx: usize) -> Option<String> {
        match self {
            DbBitPageVec::AllZeroes => None,
            DbBitPageVec::Sparse(pages) => {
                // do binary search for page_idx...
                if let Ok(matching_index) = pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                    // clear bit at the matching index
                    let bit_page = &pages[matching_index].bit_page;
                    let bit_page_in_binary = format!("{:b}", bit_page);
                    return Some(bit_page_in_binary);
                }
                None
            }
        }
    }

    #[inline]
    pub fn is_bit_set(&self, page_idx: usize, bit_idx: usize) -> bool {
        match self {
            DbBitPageVec::AllZeroes => false,
            DbBitPageVec::Sparse(pages) => {
                if let Ok(matching_index) = pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                    return BitPage::is_bit_set(&pages[matching_index].bit_page, bit_idx);
                }

                false
            }
        }
    }
}

impl fmt::Debug for DbBitPageVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            DbBitPageVec::AllZeroes => write!(f, "DbBitPageVec::AllZeroes"),
            DbBitPageVec::Sparse(pages) => write!(
                f,
                "DbBitPageVec::SparseWithZeroes(len={}, active_bits={}, start_page={:?}, end_page={:?}",
                pages.len(),
                BitPageVec::count_ones(Some(pages)),
                BitPageVec::start_page(Some(pages)),
                BitPageVec::end_page(Some(pages)),
            ),
        }
    }
}
