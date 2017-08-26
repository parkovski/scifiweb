extern crate atomic;
extern crate futures;
#[macro_use]
extern crate log;
extern crate scifi_model as model;
extern crate scifi_util as util;

mod cache;
//mod cache_access;
mod mem_access;

//pub use self::cache_access::{CacheAccessor, CacheExpireMode};
pub use self::mem_access::MemoryAccessor;
