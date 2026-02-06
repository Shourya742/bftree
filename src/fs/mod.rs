use std::sync::atomic::{AtomicUsize, Ordering};

use crate::DISK_PAGE_SIZE;

mod memory_vfs;
mod std_vfs;

/// Similar to `std::io::Write` and `std::io::Read`, but without &mut self, i.e, no locking
pub(crate) trait VfsImpl: Send + Sync {
    fn read(&self, offset: usize, buf: &mut [u8]);

    fn write(&self, offset: usize, buf: &[u8]);

    /// Allocate a new page returns the physical offset of the page
    /// The size of the page is a multiple of DISK_PAGE_SIZE
    fn alloc_offset(&self, size: usize) -> usize;

    /// When we no longer need a page, we let fs know so it can be reused.
    fn dealloc_offset(&self, offset: usize);

    /// Flush the data to disk, similar to fsync on Linux.
    fn flush(&self);

    fn open(path: impl AsRef<std::path::Path>) -> Self
    where
        Self: Sized;
}

pub(crate) fn buffer_alloc(layout: std::alloc::Layout) -> *mut u8 {
    unsafe { std::alloc::alloc(layout) }
}

pub(crate) fn buffer_dealloc(ptr: *mut u8, layout: std::alloc::Layout) {
    unsafe {
        std::alloc::dealloc(ptr, layout);
    }
}

/// A simple page allocator for disk
pub(crate) struct OffsetAlloc {
    next_available_offset: AtomicUsize,
}

impl OffsetAlloc {
    pub(crate) fn new_with(mut offset: usize) -> Self {
        if offset < DISK_PAGE_SIZE {
            // the file was empty we start from the second page
            offset = DISK_PAGE_SIZE;
        }
        Self {
            next_available_offset: AtomicUsize::new(offset),
        }
    }

    pub(crate) fn alloc(&self, size: usize) -> usize {
        self.next_available_offset.fetch_add(size, Ordering::AcqRel)
    }

    pub(crate) fn dealloc_offset(&self, _offset: usize) {
        // we don't need to dod anything here.
    }
}
