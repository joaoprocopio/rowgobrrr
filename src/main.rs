#![deny(clippy::all)]

use fxhash::{FxBuildHasher, FxHashMap};
use libc;
use std::env::{args, current_dir};
use std::fs::File;
use std::io;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::ptr::null_mut as null_mut_ptr;
use std::slice;

#[derive(Debug)]
struct Status {
    min: f64,
    max: f64,
    sum: f64,
    count: usize,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            min: f64::MAX,
            max: f64::MIN,
            sum: 0.,
            count: 0,
        }
    }
}

const NEW_LINE: u8 = b"\n"[0];
const SEPARATOR: &'static str = ";";

fn main() {
    let mut statuses =
        FxHashMap::<&str, Status>::with_capacity_and_hasher(2048, FxBuildHasher::default());

    let file = args()
        .nth(1)
        .expect("measurements file path should be provider");
    let file = current_dir()
        .and_then(|path| path.join(file).canonicalize())
        .and_then(|path| File::open(path))
        .unwrap();

    mmap(&file)
        .split(|&byte| byte == NEW_LINE)
        .filter(|&byte| !byte.is_empty())
        .for_each(|line| {
            let line = unsafe { str::from_utf8_unchecked(line) };
            let (station, temperature) = line.split_once(SEPARATOR).unwrap();
            let temperature: f64 = temperature.parse().unwrap();

            let status = statuses.entry(station).or_default();

            status.max = temperature.max(status.max);
            status.min = temperature.min(status.min);
            status.sum += temperature;
            status.count += 1;
        });

    let mut sorted = statuses.keys().collect::<Vec<_>>();
    sorted.sort_unstable();

    let mut sorted = sorted.into_iter().peekable();

    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    write!(writer, "{{").unwrap();

    while let Some(station) = sorted.next() {
        let status = statuses.get(station).unwrap();

        write!(
            writer,
            "{}={:.1}/{:.1}/{:.1}",
            station,
            status.min,
            status.sum / status.count as f64,
            status.max
        )
        .unwrap();

        if let Some(_) = sorted.peek() {
            write!(writer, ", ").unwrap();
        }
    }

    write!(writer, "}}").unwrap();
}

fn mmap(file: &File) -> &[u8] {
    let len = file.metadata().unwrap().len() as libc::size_t;

    unsafe {
        let ptr = libc::mmap(
            null_mut_ptr(),
            len,
            libc::PROT_READ,
            libc::MAP_PRIVATE,
            file.as_raw_fd(),
            0,
        );

        if ptr == libc::MAP_FAILED {
            panic!("{:?}", io::Error::last_os_error());
        }

        if libc::madvise(ptr, len, libc::MADV_SEQUENTIAL) != 0 {
            panic!("{:?}", io::Error::last_os_error());
        }

        slice::from_raw_parts(ptr as *const u8, len as usize)
    }

    // TODO: maybe munmap? it's needed if we're just shutting down the program?
}
