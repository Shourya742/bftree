use std::sync::atomic::AtomicU32;

mod platform {
    use std::sync::atomic::AtomicU32;

    #[inline]
    pub fn wait(a: &AtomicU32, expected: u32) {
        unsafe {
            libc::syscall(
                libc::SYS_futex,
                a,
                libc::FUTEX_WAIT | libc::FUTEX_PRIVATE_FLAG,
                expected,
                core::ptr::null::<libc::timespec>(),
            );
        }
    }

    #[inline]
    pub fn wake_one(ptr: *const AtomicU32) {
        unsafe {
            libc::syscall(
                libc::SYS_futex,
                ptr,
                libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG,
                1i32,
            );
        }
    }

    #[inline]
    pub fn wake_all(ptr: *const AtomicU32) {
        unsafe {
            libc::syscall(
                libc::SYS_futex,
                ptr,
                libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG,
                i32::MAX,
            );
        }
    }
}

/// If the value is value, wait until woken up.
///
/// This function might also return spuriously,
/// without a corresponding wake operation
#[inline]
pub fn wait(atomic: &AtomicU32, value: u32) {
    platform::wait(atomic, value);
}

/// Wake one thread that is waiting on this atomic
///
/// It's okay if the pointer dangles or is null
#[inline]
pub fn wake_one(atomic: *const AtomicU32) {
    platform::wake_one(atomic);
}

/// Wake all threads that are waiting on this atomic.
///
/// It's okay if the pointer dangles or is null.
#[inline]
pub fn wake_all(atomic: *const AtomicU32) {
    platform::wake_all(atomic);
}
