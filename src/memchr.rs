use libc;

pub fn memchr<'a>(haystack: &'a [u8], needle: u8) -> Option<usize> {
    if haystack.is_empty() {
        return None;
    }

    let ptr = unsafe {
        libc::memchr(
            haystack.as_ptr() as *const libc::c_void,
            needle as libc::c_int,
            haystack.len() as libc::size_t,
        )
    };

    if ptr.is_null() {
        return None;
    }

    let index = unsafe { ptr.offset_from(haystack.as_ptr() as *const libc::c_void) };

    Some(index as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        assert_eq!(memchr(b"", b'.'), None);
    }

    #[test]
    fn not_found() {
        assert_eq!(memchr(b"foobar", b'z'), None);
    }

    #[test]
    fn newline_found() {
        assert_eq!(memchr(b"foo\n", b'\n'), Some(3));
    }

    #[test]
    fn last_byte_found() {
        assert_eq!(memchr(b"foobarbaz", b'z'), Some(8));
    }

    #[test]
    fn single_byte_found() {
        assert_eq!(memchr(b"x", b'x'), Some(0));
    }

    #[test]
    fn single_byte_not_found() {
        assert_eq!(memchr(b"x", b'h'), None);
    }
}
