use std::{
    cell::RefCell,
    ffi::CString,
    fs::{self, File},
    io::Write,
    os::{fd::FromRawFd, unix::ffi::OsStrExt},
    path::{Path, PathBuf},
};

use io_uring::{IoUring, opcode};
use std::os::fd::AsRawFd;

use crate::{
    fs::{OffsetAlloc, VfsImpl},
    utils,
};

/// The purpose of this struct is to create a group of rings that share the same kernel pool thread
struct IoUringInstance {
    ring: Vec<RefCell<IoUring>>,
}

impl IoUringInstance {
    fn new(poll: bool) -> Self {
        let parallelism: usize = std::thread::available_parallelism().unwrap().into();
        let thread_cnt = 32.max(parallelism * 4);
        let mut ring = Vec::with_capacity(thread_cnt);

        for i in 0..thread_cnt {
            let mut r = IoUring::builder();

            if poll {
                r.setup_sqpoll(50000);
                r.setup_iopoll();

                if i >= 1 {
                    let pre_r: &RefCell<IoUring> = &ring[i - 1];
                    r.setup_attach_wq(pre_r.borrow().as_raw_fd());
                }
            }

            let r = r.build(8).expect("Failed to create io_uring");
            ring.push(RefCell::new(r));
        }

        Self { ring }
    }

    fn get_current_ring(&self) -> &RefCell<IoUring> {
        let v = utils::thread_id_to_u64(std::thread::current().id());
        let idx = v % self.ring.len() as u64;
        let ring = self.get_ring(idx);
        ring
    }

    fn get_ring(&self, thread_id: u64) -> &RefCell<IoUring> {
        &self.ring[thread_id as usize]
    }
}

pub(crate) struct IoUringVfs {
    pub(crate) file: File,
    offset_alloc: OffsetAlloc,
    rings: IoUringInstance,
    _path: PathBuf,
    polling: bool,
}

unsafe impl Send for IoUringVfs {}
unsafe impl Sync for IoUringVfs {}

impl IoUringVfs {
    pub(crate) fn new_blocking(path: impl AsRef<Path>) -> Self {
        Self::new_inner(path, false)
    }

    fn wait_cnt(&self) -> usize {
        if self.polling { 0 } else { 1 }
    }

    fn new_inner(path: impl AsRef<Path>, use_poll: bool) -> Self {
        let path = path.as_ref();
        let parent = path.parent().unwrap();
        _ = std::fs::create_dir_all(parent);
        let path_cstr = CString::new(path.as_os_str().as_bytes()).unwrap();
        let raw_fd = unsafe {
            libc::open(
                path_cstr.as_ptr(),
                libc::O_DIRECT
                    | libc::O_RDWR
                    | libc::O_CREAT
                    | libc::S_IRUSR as i32
                    | libc::S_IWUSR as i32,
            )
        };
        assert!(
            raw_fd >= 0,
            "Failed to open file {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        );

        let mut file = unsafe { File::from_raw_fd(raw_fd) };
        file.flush().unwrap();

        let offset = file.metadata().unwrap().len();
        IoUringVfs {
            file,
            offset_alloc: OffsetAlloc::new_with(offset as usize),
            rings: IoUringInstance::new(use_poll),
            _path: path.to_path_buf(),
            polling: use_poll,
        }
    }
}

impl VfsImpl for IoUringVfs {
    fn alloc_offset(&self, size: usize) -> usize {
        self.offset_alloc.alloc(size)
    }

    fn dealloc_offset(&self, offset: usize) {
        self.offset_alloc.dealloc_offset(offset);
    }

    fn open(path: impl AsRef<Path>) -> Self {
        Self::new_inner(path, true)
    }

    fn flush(&self) {
        self.file.sync_all().unwrap();
    }

    fn read(&self, offset: usize, buf: &mut [u8]) {
        let read_e = opcode::Read::new(
            io_uring::types::Fd(self.file.as_raw_fd()),
            buf.as_mut_ptr(),
            buf.len() as _,
        )
        .offset(offset as u64)
        .build()
        .user_data(0x42);
        let ring = self.rings.get_current_ring();
        let mut ring_mut = ring.borrow_mut();
        unsafe {
            let mut sq = ring_mut.submission();
            sq.push(&read_e).unwrap();
            sq.sync();
        }
        ring_mut.submit_and_wait(self.wait_cnt()).unwrap();
        let mut cq = ring_mut.completion();
        loop {
            cq.sync();
            match cq.next() {
                Some(cqe) => {
                    assert_eq!(cqe.user_data(), 0x42);
                    assert_eq!(
                        cqe.result(),
                        buf.len() as i32,
                        "Read cqe result error: {}",
                        std::io::Error::last_os_error()
                    );
                    break;
                }
                None => {
                    continue;
                }
            }
        }
    }

    fn write(&self, offset: usize, buf: &[u8]) {
        let write_e = opcode::Write::new(
            io_uring::types::Fd(self.file.as_raw_fd()),
            buf.as_ptr(),
            buf.len() as _,
        )
        .offset(offset as u64)
        .build()
        .user_data(0x42);
        let ring = self.rings.get_current_ring();
        let mut ring_mut = ring.borrow_mut();
        unsafe {
            let mut sq = ring_mut.submission();
            sq.push(&write_e).expect("submission queue is full");
            sq.sync();
        }
        ring_mut.submit_and_wait(self.wait_cnt()).unwrap();
        let mut cq = ring_mut.completion();

        loop {
            cq.sync();
            match cq.next() {
                Some(cqe) => {
                    assert_eq!(cqe.user_data(), 0x42);
                    assert_eq!(
                        cqe.result(),
                        buf.len() as i32,
                        "Write cqe result error: {}",
                        std::io::Error::last_os_error()
                    );
                    break;
                }
                None => {
                    continue;
                }
            }
        }
    }
}
