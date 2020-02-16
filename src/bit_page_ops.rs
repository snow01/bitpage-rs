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
        match (&mut *self, second) {
            (BitPage::Zeroes, _) | (_, BitPage::Ones) => {
                // do nothing
            }

            (_, BitPage::Zeroes) => *self = BitPage::Zeroes,
            (BitPage::Ones, _) => *self = second.clone(),
            (BitPage::Some(value_1), BitPage::Some(value_2)) => *value_1 &= value_2,
            #[allow(unreachable_patterns)]
            _ => {}
        }
    }

    #[inline]
    pub fn or(&mut self, second: &BitPage) {
        match (&mut *self, second) {
            (BitPage::Ones, _) | (_, BitPage::Zeroes) => {
                // do nothing
            }
            (_, BitPage::Ones) => *self = BitPage::Ones,
            (BitPage::Zeroes, _) => *self = second.clone(),
            (BitPage::Some(value_1), BitPage::Some(value_2)) => *value_1 |= value_2,
            #[allow(unreachable_patterns)]
            _ => {}
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
