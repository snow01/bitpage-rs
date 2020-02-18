use std::fmt;

use crate::BitPage;

#[derive(Clone)]
pub enum BitPageVec {
    AllZeroes,
    Sparse(Vec<BitPageWithPosition>),
}

#[derive(Clone, Debug)]
pub struct BitPageWithPosition {
    pub(crate) page_idx: usize,
    pub(crate) bit_page: u64,
}

impl Default for BitPageVec {
    fn default() -> Self {
        BitPageVec::all_zeros()
    }
}

impl BitPageVec {
    #[inline]
    pub fn all_zeros() -> BitPageVec {
        BitPageVec::AllZeroes
    }

    #[inline]
    pub fn all_ones(num_pages: usize) -> BitPageVec {
        let mut bit_page = BitPageVec::AllZeroes;
        bit_page.not(num_pages);
        bit_page
    }

    #[inline]
    pub fn clear_bit(&mut self, page_idx: usize, bit_idx: usize) {
        match self {
            BitPageVec::AllZeroes => {
                // no-op
            }
            BitPageVec::Sparse(pages) => {
                // do binary search for page_idx...
                if let Ok(matching_index) = pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                    // clear bit at the matching index
                    let bit_page = &mut pages[matching_index].bit_page;
                    BitPage::clear_bit(bit_page, bit_idx);

                    if BitPage::is_zero(bit_page) {
                        // remove this bit page from matching index and compact page
                        pages.remove(matching_index);
                    }

                    // compact BitPageVec
                    if pages.is_empty() {
                        *self = BitPageVec::all_zeros();
                    }
                }
            }
        }
    }

    #[inline]
    pub fn set_bit(&mut self, page_idx: usize, bit_idx: usize) {
        match self {
            BitPageVec::AllZeroes => {
                let mut bit_page = BitPage::zeroes();
                BitPage::set_bit(&mut bit_page, bit_idx);

                *self = BitPageVec::Sparse(vec![BitPageWithPosition { page_idx, bit_page }]);
            }
            BitPageVec::Sparse(pages) => {
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
    pub fn is_bit_set(&self, page_idx: usize, bit_idx: usize) -> bool {
        match self {
            BitPageVec::AllZeroes => false,
            BitPageVec::Sparse(pages) => {
                if let Ok(matching_index) = pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                    return BitPage::is_bit_set(&pages[matching_index].bit_page, bit_idx);
                }

                false
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            BitPageVec::AllZeroes => 0,
            BitPageVec::Sparse(pages) => pages.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        match self {
            BitPageVec::AllZeroes => true,
            BitPageVec::Sparse(pages) => pages.len() == 0,
        }
    }

    pub fn count_ones(&self) -> u32 {
        match self {
            BitPageVec::AllZeroes => 0,
            BitPageVec::Sparse(pages) => pages.iter().map(|value| value.bit_page.count_ones()).sum(),
        }
    }

    pub fn start_page(&self) -> usize {
        match self {
            BitPageVec::AllZeroes => 0,
            BitPageVec::Sparse(pages) => pages[0].page_idx,
        }
    }

    pub fn end_page(&self) -> usize {
        match self {
            BitPageVec::AllZeroes => 0,
            BitPageVec::Sparse(pages) => pages[self.len() - 1].page_idx,
        }
    }
}

impl fmt::Debug for BitPageVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            BitPageVec::AllZeroes => write!(f, "BitPageVec::AllZeroes"),
            BitPageVec::Sparse(pages) => write!(
                f,
                "BitPageVec::Sparse(len={}, active_bits={}, start_page={}, end_page={}, pages={:?})",
                self.len(),
                self.count_ones(),
                self.start_page(),
                self.end_page(),
                pages
            ),
        }
    }
}
