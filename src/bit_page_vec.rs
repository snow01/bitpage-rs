// @author shailendra.sharma
use std::fmt;

use crate::bit_page::BitPageWithPosition;
use crate::BitPage;

#[derive(Copy, Clone, Debug)]
pub enum BitPageVecKind {
    AllZeroes,
    SparseWithZeroesHole,
    AllOnes,
    SparseWithOnesHole,
}

#[derive(Clone)]
pub struct BitPageVec {
    pub(crate) kind: BitPageVecKind,
    pub(crate) pages: Option<Vec<BitPageWithPosition>>,
    pub(crate) last_bit_index: (usize, usize),
}

impl BitPageVec {
    pub fn new(kind: BitPageVecKind, pages: Option<Vec<BitPageWithPosition>>, last_bit_index: (usize, usize)) -> BitPageVec {
        BitPageVec {
            kind,
            pages,
            last_bit_index,
        }
    }

    #[inline]
    pub fn all_zeros(last_bit_index: (usize, usize)) -> BitPageVec {
        BitPageVec::new(BitPageVecKind::AllZeroes, None, last_bit_index)
    }

    #[inline]
    pub fn all_ones(last_bit_index: (usize, usize)) -> BitPageVec {
        BitPageVec::new(BitPageVecKind::AllOnes, None, last_bit_index)
    }

    pub fn kind(&self) -> &BitPageVecKind {
        &self.kind
    }

    pub fn last_bit_index(&self) -> (usize, usize) {
        self.last_bit_index
    }

    #[inline]
    pub fn clear_bit(&mut self, page_idx: usize, bit_idx: usize) {
        match self.kind {
            BitPageVecKind::AllZeroes => {
                // no-op
            }
            BitPageVecKind::AllOnes => {
                let mut bit_page = BitPage::ones();
                BitPage::clear_bit(&mut bit_page, bit_idx);

                *self = BitPageVec::new(
                    BitPageVecKind::SparseWithOnesHole,
                    Some(vec![BitPageWithPosition { page_idx, bit_page }]),
                    self.last_bit_index,
                );
            }
            BitPageVecKind::SparseWithZeroesHole => {
                if let Some(ref mut pages) = self.pages {
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
                            *self = BitPageVec::all_zeros(self.last_bit_index);
                        }
                    }
                }
            }
            BitPageVecKind::SparseWithOnesHole => {
                if let Some(ref mut pages) = self.pages {
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
                                *self = BitPageVec::all_zeros(self.last_bit_index);
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
    }

    #[inline]
    pub fn set_bit(&mut self, page_idx: usize, bit_idx: usize) {
        match self.kind {
            BitPageVecKind::AllZeroes => {
                let mut bit_page = BitPage::zeroes();
                BitPage::set_bit(&mut bit_page, bit_idx);

                *self = BitPageVec::new(
                    BitPageVecKind::SparseWithZeroesHole,
                    Some(vec![BitPageWithPosition { page_idx, bit_page }]),
                    self.last_bit_index,
                );
            }
            BitPageVecKind::AllOnes => {
                // NO-OP
            }
            BitPageVecKind::SparseWithZeroesHole => {
                if let Some(ref mut pages) = self.pages {
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
            BitPageVecKind::SparseWithOnesHole => {
                if let Some(ref mut pages) = self.pages {
                    // do binary search for page_idx...
                    if let Ok(matching_index) = pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                        // set bit at the matching index
                        let bit_page = &mut pages[matching_index].bit_page;
                        BitPage::set_bit(bit_page, bit_idx);
                    }
                }
            }
        }
    }

    #[inline]
    pub fn is_bit_set(&self, page_idx: usize, bit_idx: usize) -> bool {
        match self.kind {
            BitPageVecKind::AllZeroes => false,
            BitPageVecKind::AllOnes => true,
            BitPageVecKind::SparseWithZeroesHole => {
                if let Some(ref pages) = self.pages {
                    if let Ok(matching_index) = pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                        return BitPage::is_bit_set(&pages[matching_index].bit_page, bit_idx);
                    }
                }

                false
            }
            BitPageVecKind::SparseWithOnesHole => {
                if let Some(ref pages) = self.pages {
                    match pages.binary_search_by(|probe| probe.page_idx.cmp(&page_idx)) {
                        Ok(matching_index) => BitPage::is_bit_set(&pages[matching_index].bit_page, bit_idx),
                        Err(_) => true,
                    }
                } else {
                    false
                }
            }
        }
    }

    pub fn size(&self) -> usize {
        self.pages.as_ref().map_or_else(|| 0, |pages| pages.len())
    }

    pub(crate) fn count_ones(pages: Option<&Vec<BitPageWithPosition>>) -> u32 {
        pages.map_or_else(|| 0, |pages| pages.iter().map(|value| value.bit_page.count_ones()).sum())
    }

    pub(crate) fn start_page(pages: Option<&Vec<BitPageWithPosition>>) -> Option<usize> {
        pages.and_then(|pages| pages.get(0)).map(|page| page.page_idx)
    }

    pub(crate) fn end_page(pages: Option<&Vec<BitPageWithPosition>>) -> Option<usize> {
        pages.and_then(|pages| pages.get(pages.len() - 1)).map(|page| page.page_idx)
    }
}

impl fmt::Debug for BitPageVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self.kind {
            BitPageVecKind::AllZeroes => write!(f, "BitPageVec::AllZeroes"),
            BitPageVecKind::AllOnes => write!(f, "BitPageVec::AllOnes"),
            BitPageVecKind::SparseWithZeroesHole => write!(
                f,
                "BitPageVec::SparseWithZeroes(len={}, active_bits={}, start_page={:?}, end_page={:?}",
                self.size(),
                BitPageVec::count_ones(self.pages.as_ref()),
                BitPageVec::start_page(self.pages.as_ref()),
                BitPageVec::end_page(self.pages.as_ref()),
            ),
            BitPageVecKind::SparseWithOnesHole => write!(
                f,
                "BitPageVec::SparseWithOnesHole(len={}, active_bits={}, start_page={:?}, end_page={:?}",
                self.size(),
                BitPageVec::count_ones(self.pages.as_ref()),
                BitPageVec::start_page(self.pages.as_ref()),
                BitPageVec::end_page(self.pages.as_ref()),
            ),
        }
    }
}
