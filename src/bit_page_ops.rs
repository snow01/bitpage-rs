use crate::bit_page::BitPage;

impl BitPage {
    #[inline]
    pub fn not(&mut self) {
        match self {
            BitPage::Zeroes => *self = BitPage::Ones,
            BitPage::Ones => *self = BitPage::Zeroes,
            BitPage::Some(value) => *value = !*value,
        }
    }

    #[inline]
    pub fn and(&mut self, second: &BitPage) {
        match &mut *self {
            BitPage::Zeroes => {
                // do nothing
            }
            BitPage::Ones => {
                *self = second.clone();
            }
            BitPage::Some(value_1) => match second {
                BitPage::Zeroes => {
                    *self = BitPage::Zeroes;
                }
                BitPage::Ones => {
                    // do nothing
                }
                BitPage::Some(value_2) => {
                    *value_1 &= value_2;
                    if 0.eq(value_1) {
                        *self = BitPage::Zeroes;
                    }
                }
            },
        }
    }

    #[inline]
    pub fn or(&mut self, second: &BitPage) {
        match &mut *self {
            BitPage::Zeroes => {
                *self = second.clone();
            }
            BitPage::Ones => {
                // do nothing
            }
            BitPage::Some(value_1) => match second {
                BitPage::Zeroes => {
                    // do nothing
                }
                BitPage::Ones => {
                    *self = second.clone();
                }
                BitPage::Some(value_2) => {
                    *value_1 |= value_2;
                }
            },
        }
    }

    #[inline]
    pub fn is_all_zeros(&self) -> bool {
        match self {
            BitPage::Zeroes => true,
            BitPage::Ones => false,
            BitPage::Some(value) => 0.eq(value),
        }
    }

    #[inline]
    pub fn is_all_ones(&self) -> bool {
        match self {
            BitPage::Zeroes => false,
            BitPage::Ones => true,
            BitPage::Some(value) => u64::max_value().eq(value),
        }
    }
}
