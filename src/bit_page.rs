pub struct BitPage;

const ZERO: u64 = 0 as u64;

impl BitPage {
    #[inline]
    pub fn zeroes() -> u64 {
        0
    }

    #[inline]
    pub fn ones() -> u64 {
        u64::max_value()
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
    pub fn is_zero(value: &u64) -> bool {
        ZERO.eq(value)
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
