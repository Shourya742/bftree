use std::{
    fs::{File, OpenOptions},
    os::unix::fs::FileExt,
    path::PathBuf,
};

use crate::fs::{OffsetAlloc, VfsImpl};

pub(crate) struct StdVfs {
    file: File,
    offset_alloc: OffsetAlloc,
    _path: PathBuf,
}

impl VfsImpl for StdVfs {
    fn alloc_offset(&self, size: usize) -> usize {
        self.offset_alloc.alloc(size)
    }

    fn open(path: impl AsRef<std::path::Path>) -> Self
    where
        Self: Sized,
    {
        let path = path.as_ref().to_path_buf();
        let parent = path.parent().unwrap();
        _ = std::fs::create_dir_all(parent);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .unwrap();
        let offset = file.metadata().unwrap().len();
        Self {
            file,
            offset_alloc: OffsetAlloc::new_with(offset as usize),
            _path: path.to_path_buf(),
        }
    }

    fn dealloc_offset(&self, offset: usize) {
        self.offset_alloc.dealloc_offset(offset);
    }

    fn flush(&self) {
        self.file.sync_all().unwrap()
    }

    fn read(&self, offset: usize, buf: &mut [u8]) {
        self.file.read_at(buf, offset as u64).unwrap();
    }

    fn write(&self, offset: usize, buf: &[u8]) {
        self.file.write_at(buf, offset as u64).unwrap();
    }
}
