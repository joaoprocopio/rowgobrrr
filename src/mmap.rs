use libc;
use std::io;
use std::marker;
use std::ptr;
use std::slice;

#[derive(Debug)]
pub struct Mmap<'a> {
    ptr: *mut libc::c_void,
    len: libc::size_t,
    _buf: marker::PhantomData<&'a [u8]>,
}

impl<'a> Mmap<'a> {
    pub fn new(
        len: libc::size_t,
        prot: libc::c_int,
        flags: libc::c_int,
        fd: libc::c_int,
        offset: libc::off_t,
    ) -> io::Result<Self> {
        // TODO: check if page is aligned with the OS
        let ptr = unsafe {
            let ptr = libc::mmap(ptr::null_mut(), len, prot, flags, fd, offset);

            if ptr == libc::MAP_FAILED {
                return Err(io::Error::last_os_error());
            }

            ptr
        };

        Ok(Self {
            ptr: ptr,
            len: len,
            _buf: marker::PhantomData,
        })
    }

    pub fn advise(&self, advice: libc::c_int) -> io::Result<()> {
        let res = unsafe { libc::madvise(self.ptr, self.len, advice) };

        if res == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    pub fn as_slice(&self) -> &'a [u8] {
        unsafe { slice::from_raw_parts(self.ptr as *const u8, self.len) }
    }
}

impl<'a> Drop for Mmap<'a> {
    fn drop(&mut self) {
        // this could fail silently...
        if !self.ptr.is_null() && self.len > 0 {
            unsafe {
                libc::munmap(self.ptr, self.len);
            }
        }
    }
}
