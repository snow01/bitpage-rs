#[macro_use]
extern crate lazy_static;

pub use bit_page::BitPage;
pub use bit_page_iter::{MAX_BITS as BIT_PAGE_MAX_BITS, NUM_BYTES as BIT_PAGE_NUM_BYTES};
pub use bit_page_vec::BitPageVec;
pub use bit_page_vec_ops::{BooleanOp, BooleanOpLeaf};

// bit page and its associated modules
mod bit_page;
mod bit_page_iter;
mod bit_page_ops;
mod bit_page_serde;

// bit page vector and its associated modules
mod bit_page_vec;
mod bit_page_vec_iter;
mod bit_page_vec_ops;
mod bit_page_vec_serde;
