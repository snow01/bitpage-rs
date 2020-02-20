use std::ops::Range;
use std::slice::Iter;

use arrayvec::ArrayVec;

use crate::bit_page::BitPage;

pub enum BitPageActiveBitsIterator {
    AllZeroes,
    AllOnes { range: Range<usize> },
    Some { iter: Box<dyn Iterator<Item = usize>> },
}

impl<'a> Iterator for BitPageActiveBitsIterator {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            BitPageActiveBitsIterator::AllZeroes => None,
            BitPageActiveBitsIterator::AllOnes { range } => range.next(),
            BitPageActiveBitsIterator::Some { iter } => iter.next(),
        }
    }
}

impl BitPage {
    pub fn active_bits(value: u64) -> BitPageActiveBitsIterator {
        match value {
            BitPage::MIN_VALUE => BitPageActiveBitsIterator::AllZeroes,
            BitPage::MAX_VALUE => BitPageActiveBitsIterator::AllOnes {
                range: (0..BitPage::MAX_BITS),
            },
            _ => {
                let mut byte_masks = Vec::<u8>::with_capacity(BitPage::NUM_BYTES);
                for i in 0..BitPage::NUM_BYTES {
                    let byte = (value >> (i * 8)) as u8;
                    byte_masks.push(byte);
                }

                let iter = byte_masks
                    .into_iter()
                    .enumerate()
                    .flat_map(|(byte_idx, byte_mask)| active_bits_iter(byte_mask).map(move |bit_idx| byte_idx * 8 + *bit_idx));

                BitPageActiveBitsIterator::Some { iter: Box::new(iter) }
            }
        }
    }
}

const ACTIVE_BITS_LEN: usize = u8::max_value() as usize + 1;
type ActiveBitsType = ArrayVec<[Vec<usize>; ACTIVE_BITS_LEN]>;

lazy_static! {
    static ref ACTIVE_BITS: ActiveBitsType = build_byte_to_active_bits();
}

fn build_byte_to_active_bits() -> ActiveBitsType {
    let mut array = ArrayVec::<[_; ACTIVE_BITS_LEN]>::new();

    for i in 0..ACTIVE_BITS_LEN {
        array.push(build_active_bits(i as u8));
    }

    array
}

fn build_active_bits(mut bit: u8) -> Vec<usize> {
    let mut index = 0;
    let mut bits = Vec::new();
    while bit != 0 {
        if bit & 1 == 1 {
            bits.push(index);
        }

        bit >>= 1;
        index += 1;
    }

    bits
}

fn active_bits_iter(byte: u8) -> Iter<'static, usize> {
    ACTIVE_BITS[byte as usize].iter()
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use crate::BitPage;

    #[test]
    fn test_ops() {
        println!("ALL ZEROS -- SET BIT");

        for i in 0..64 {
            let mut bit_page = BitPage::zeroes();

            BitPage::set_bit(&mut bit_page, i);

            println!(
                "BitPage[{}] = {:?} ==> {} ==> {:?}",
                i,
                bit_page,
                BitPage::is_bit_set(&bit_page, i),
                BitPage::active_bits(bit_page).collect_vec()
            );
        }
    }
}
