use std::fmt;

use crate::BitPage;

#[derive(Clone)]
pub enum BitPageVec {
    AllZeroes,
    SparseWithZeroesHole(Vec<BitPageWithPosition>),

    // new additions...
    AllOnes,
    SparseWithOnesHole(Vec<BitPageWithPosition>),
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
    pub fn all_ones() -> BitPageVec {
        //        let mut bit_page = BitPageVec::AllZeroes;
        //        bit_page.not(num_pages);
        //        bit_page
        BitPageVec::AllOnes
    }

    #[inline]
    pub fn clear_bit(&mut self, page_idx: usize, bit_idx: usize) {
        match self {
            BitPageVec::AllZeroes => {
                // no-op
            }
            BitPageVec::AllOnes => {
                let mut bit_page = BitPage::ones();
                BitPage::clear_bit(&mut bit_page, bit_idx);

                *self = BitPageVec::SparseWithOnesHole(vec![BitPageWithPosition { page_idx, bit_page }]);
            }
            BitPageVec::SparseWithZeroesHole(pages) => {
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
                        *self = BitPageVec::all_zeros();
                    }
                }
            }
            BitPageVec::SparseWithOnesHole(pages) => {
                // do binary search for page_idx...
                match pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                    Ok(matching_index) => {
                        // clear bit at the matching index
                        let bit_page = &mut pages[matching_index].bit_page;
                        BitPage::clear_bit(bit_page, bit_idx);

                        if BitPage::is_zeroes(bit_page) {
                            // remove this bit page from matching index and compact page
                            pages.remove(matching_index);
                        }

                        // compact BitPageVec
                        if pages.is_empty() {
                            *self = BitPageVec::all_zeros();
                        }
                    }
                    Err(insertion_index) => {
                        let mut bit_page = BitPage::ones();
                        BitPage::clear_bit(&mut bit_page, bit_idx);

                        pages.insert(insertion_index, BitPageWithPosition { page_idx, bit_page });
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

                *self = BitPageVec::SparseWithZeroesHole(vec![BitPageWithPosition { page_idx, bit_page }]);
            }
            BitPageVec::AllOnes => {
                // NO-OP
            }
            BitPageVec::SparseWithZeroesHole(pages) => {
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
            BitPageVec::SparseWithOnesHole(pages) => {
                // do binary search for page_idx...
                if let Ok(matching_index) = pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                    // set bit at the matching index
                    let bit_page = &mut pages[matching_index].bit_page;
                    BitPage::set_bit(bit_page, bit_idx);
                }
            }
        }
    }

    #[inline]
    pub fn is_bit_set(&self, page_idx: usize, bit_idx: usize) -> bool {
        match self {
            BitPageVec::AllZeroes => false,
            BitPageVec::AllOnes => true,
            BitPageVec::SparseWithZeroesHole(pages) => {
                if let Ok(matching_index) = pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                    return BitPage::is_bit_set(&pages[matching_index].bit_page, bit_idx);
                }

                false
            }
            BitPageVec::SparseWithOnesHole(pages) => match pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                Ok(matching_index) => BitPage::is_bit_set(&pages[matching_index].bit_page, bit_idx),
                Err(_) => true,
            },
        }
    }

    pub fn size(&self) -> usize {
        match self {
            BitPageVec::AllZeroes => 0,
            BitPageVec::AllOnes => 0,
            BitPageVec::SparseWithZeroesHole(pages) => pages.len(),
            BitPageVec::SparseWithOnesHole(pages) => pages.len(),
        }
    }

    pub(crate) fn count_ones(pages: &[BitPageWithPosition]) -> u32 {
        pages.iter().map(|value| value.bit_page.count_ones()).sum()
    }

    pub(crate) fn start_page(pages: &[BitPageWithPosition]) -> usize {
        pages[0].page_idx
    }

    pub(crate) fn end_page(pages: &[BitPageWithPosition]) -> usize {
        pages[pages.len() - 1].page_idx
    }
}

impl fmt::Debug for BitPageVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            BitPageVec::AllZeroes => write!(f, "BitPageVec::AllZeroes"),
            BitPageVec::AllOnes => write!(f, "BitPageVec::AllOnes"),
            BitPageVec::SparseWithZeroesHole(pages) => write!(
                f,
                "BitPageVec::SparseWithZeroes(len={}, active_bits={}, start_page={}, end_page={}",
                self.size(),
                BitPageVec::count_ones(pages),
                BitPageVec::start_page(pages),
                BitPageVec::end_page(pages)
            ),
            BitPageVec::SparseWithOnesHole(pages) => write!(
                f,
                "BitPageVec::SparseWithZeroes(len={}, active_bits={}, start_page={}, end_page={}",
                self.size(),
                BitPageVec::count_ones(pages),
                BitPageVec::start_page(pages),
                BitPageVec::end_page(pages)
            ),
        }
    }
}
