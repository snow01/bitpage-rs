use std::fmt;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum BitPage {
    Zeroes,
    Ones,
    Some(u64),
}

impl fmt::Debug for BitPage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match &self {
            BitPage::Zeroes => write!(f, "BitPage::AllZeroes"),
            BitPage::Ones => write!(f, "BitPage::AllOnes"),
            BitPage::Some(value) => write!(f, "BitPage::Some(active={}) ==> {:b}", self.count_ones(), value),
        }
    }
}

impl Default for BitPage {
    fn default() -> Self {
        BitPage::zeroes()
    }
}

impl BitPage {
    #[inline]
    pub fn zeroes() -> BitPage {
        BitPage::Zeroes
    }

    #[inline]
    pub fn ones() -> BitPage {
        BitPage::Ones
    }

    #[inline]
    pub fn clear_bit(&mut self, bit_idx: usize) {
        match self {
            BitPage::Zeroes => {
                // no-op
            }
            BitPage::Ones => {
                let value = !get_mask(bit_idx);

                *self = BitPage::Some(value)
            }
            BitPage::Some(value) => {
                *value &= !get_mask(bit_idx);

                // compact BitPage
                if 0.eq(value) {
                    *self = BitPage::Zeroes;
                }
            }
        }
    }

    #[inline]
    pub fn set_bit(&mut self, bit_idx: usize) {
        match self {
            BitPage::Zeroes => {
                let value = get_mask(bit_idx);
                *self = BitPage::Some(value)
            }
            BitPage::Ones => {
                // no-op
            }
            BitPage::Some(value) => {
                *value |= get_mask(bit_idx);

                // compact BitPage
                if u64::max_value().eq(value) {
                    *self = BitPage::Ones;
                }
            }
        }
    }

    #[inline]
    pub fn is_bit_set(&self, bit_idx: usize) -> bool {
        match self {
            BitPage::Zeroes => false,
            BitPage::Ones => true,
            BitPage::Some(value) => {
                let value_mask = get_mask(bit_idx);

                value & value_mask > 0
            }
        }
    }

    pub fn count_ones(&self) -> u32 {
        match self {
            BitPage::Zeroes => 0,
            BitPage::Ones => 64,
            BitPage::Some(value) => value.count_ones(),
        }
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

#[cfg(test)]
mod tests {
    use crate::bit_page::BitPage;

    #[test]
    fn test_ops() {
        println!("ALL ONES -- CLEAR BIT");
        let mut bit_page = BitPage::ones();

        for i in 0..64 {
            bit_page.clear_bit(i);

            println!("BitPage[{}] = {:?}", i, bit_page);
        }

        println!("ALL ONES -- SET BIT");
        let mut bit_page = BitPage::ones();

        for i in 0..64 {
            bit_page.set_bit(i);

            println!("BitPage[{}] = {:?}", i, bit_page);
        }

        println!("ALL ZEROS -- CLEAR BIT");
        let mut bit_page = BitPage::zeroes();

        for i in 0..64 {
            bit_page.clear_bit(i);

            println!("BitPage[{}] = {:?} ==> {}", i, bit_page, bit_page.is_bit_set(i));
        }

        println!("ALL ZEROS -- SET BIT");
        let mut bit_page = BitPage::zeroes();

        for i in 0..64 {
            bit_page.set_bit(i);

            println!("BitPage[{}] = {:?} ==> {}", i, bit_page, bit_page.is_bit_set(i));
        }
    }
}
