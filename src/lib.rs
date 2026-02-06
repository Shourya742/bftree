#![allow(warnings)]

mod fs;
mod utils;

pub(crate) const INNER_NODE_SIZE: usize = 4096;
pub(crate) const DISK_PAGE_SIZE: usize = 4096;
pub(crate) const MAX_LEAF_PAGE_SIZE: usize = 32768; // 32 KB
pub(crate) const MAX_KEY_LEN: usize = 2020;
pub(crate) const MAX_VALUE_LEN: usize = 16332; // 16kB
pub(crate) const CACHE_LINE_SIZE: usize = 64;
pub(crate) const FENCE_KEY_CNT: usize = 2;
pub(crate) const KV_META_SIZE: usize = 8;
