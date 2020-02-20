// @author shailendra.sharma
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

fn masks_inner() -> [u64; 64] {
    let mut masks: [u64; 64] = [0; 64];

    for (index, mask) in masks.iter_mut().enumerate() {
        *mask = 0x01 << index as u64;
    }

    masks
}

#[inline]
fn masks() -> &'static [u64; 64] {
    &*MASKS
}

#[inline]
fn get_mask(bit_idx: usize) -> u64 {
    masks()[bit_idx]
}

lazy_static! {
    static ref MASKS: [u64; 64] = masks_inner();
}
