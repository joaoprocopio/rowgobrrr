use libc;
use std::io;
use std::ptr;
use std::slice;

pub struct Mmap {
    ptr: *mut libc::c_void,
    len: libc::size_t,
}

impl Mmap {
    pub fn new(len: libc::size_t, fd: libc::c_int, offset: libc::off_t) -> io::Result<Self> {
        // TODO: check if page is aligned with the OS
        let ptr = unsafe {
            // TODO: when the code is running on parallel, flags should be configured
            let ptr = libc::mmap(
                ptr::null_mut(),
                len,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                fd,
                offset,
            );

            if ptr == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }

            // TODO: advise with SEQUENTIAL and/or HUGE PAGES

            ptr
        };

        Ok(Self { ptr: ptr, len: len })
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.ptr as *const u8, self.len) }
    }
}

impl Drop for Mmap {
    fn drop(&mut self) {
        // this could fail silently...
        if !self.ptr.is_null() && self.len > 0 {
            unsafe {
                libc::munmap(self.ptr, self.len);
            }
        }
    }
}
