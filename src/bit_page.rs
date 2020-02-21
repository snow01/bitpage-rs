use std::fmt;

// @author shailendra.sharma
#[derive(Clone)]
pub struct BitPageWithPosition {
    pub(crate) page_idx: usize,
    pub(crate) bit_page: u64,
}

impl fmt::Debug for BitPageWithPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.page_idx)
    }
}

pub struct BitPage;

impl BitPage {
    pub const MIN_VALUE: u64 = 0 as u64;

    pub const MAX_VALUE: u64 = u64::max_value();

    pub const MAX_BITS: usize = 64;

    pub const NUM_BYTES: usize = Self::MAX_BITS / 8;

    #[inline]
    pub fn zeroes() -> u64 {
        Self::MIN_VALUE
    }

    #[inline]
    pub fn ones() -> u64 {
        Self::MAX_VALUE
    }

    #[inline]
    pub fn clear_bit(value: &mut u64, bit_idx: usize) {
        *value &= !get_mask(bit_idx);
    }

    #[inline]
    pub fn set_bit(value: &mut u64, bit_idx: usize) {
        *value |= get_mask(bit_idx);
    }

    #[inline]
    pub fn is_bit_set(value: &u64, bit_idx: usize) -> bool {
        let value_mask = get_mask(bit_idx);

        value & value_mask > 0
    }

    #[inline]
    pub fn count_ones(value: &u64) -> u32 {
        value.count_ones()
    }

    #[inline]
    pub fn is_zeroes(value: &u64) -> bool {
        Self::MIN_VALUE.eq(value)
    }

    #[inline]
    pub fn is_ones(value: &u64) -> bool {
        Self::MAX_VALUE.eq(value)
    }
}

fn masks_inner() -> [u64; BitPage::MAX_BITS] {
    let mut masks: [u64; BitPage::MAX_BITS] = [0; BitPage::MAX_BITS];

    for (index, mask) in masks.iter_mut().enumerate() {
        *mask = 0x01 << index as u64;
    }

    masks
}

fn zero_masks_inner() -> [u64; BitPage::MAX_BITS] {
    let mut masks: [u64; BitPage::MAX_BITS] = [0; BitPage::MAX_BITS];

    let mut mask = 0;
    for index in 0..BitPage::MAX_BITS {
        masks[index] = mask;

        BitPage::set_bit(&mut mask, index);
    }

    masks
}

#[inline]
fn masks() -> &'static [u64; BitPage::MAX_BITS] {
    &*MASKS
}

#[inline]
pub fn zero_masks() -> &'static [u64; BitPage::MAX_BITS] {
    &*ZERO_MASKS
}

#[inline]
fn get_mask(bit_idx: usize) -> u64 {
    masks()[bit_idx]
}

lazy_static! {
    static ref MASKS: [u64; BitPage::MAX_BITS] = masks_inner();
    static ref ZERO_MASKS: [u64; BitPage::MAX_BITS] = zero_masks_inner();
}

#[cfg(test)]
mod tests {
    use crate::bit_page::zero_masks_inner;

    #[test]
    fn test_zero_masks() {
        let zero_masks = zero_masks_inner();

        for mask in zero_masks.iter() {
            println!("{:b}", mask);
        }
    }
}
