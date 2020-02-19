#[macro_use]
extern crate lazy_static;

pub use bit_page::BitPage;
pub use bit_page_vec::BitPageVec;
pub use boolean_op::{BooleanOp, BooleanOpResult};

// bit page and its associated modules
mod bit_page;
mod bit_page_active_bits;
mod bit_page_serde;

// bit page vector and its associated modules
mod bit_page_vec;
mod bit_page_vec_active_bits;
mod bit_page_vec_iter;
mod bit_page_vec_ops;
mod bit_page_vec_serde;

// boolean op
mod boolean_op;
