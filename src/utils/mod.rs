mod atomic_wait;

use std::hash::Hasher;
use std::hash::{DefaultHasher, Hash};

pub(crate) fn thread_id_to_u64(tid: std::thread::ThreadId) -> u64 {
    let mut hasher = DefaultHasher::new();
    tid.hash(&mut hasher);
    hasher.finish()
}
