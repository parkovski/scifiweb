extern crate atomic;
extern crate futures;
#[macro_use]
extern crate log;
extern crate sf_model;
extern crate sf_util;

mod cache;
//mod cache_access;
mod mem_access;

//pub use self::cache_access::{CacheAccessor, CacheExpireMode};
pub use self::mem_access::MemoryAccessor;
