#![deny(clippy::all)]

use libc;
use std::collections::HashMap;
use std::env::current_dir;
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::ptr::null_mut as null_mut_ptr;
use std::slice;

/// each line inside the measurements.txt file is in the following format `<string: station_name>;<f64: measurement>`
///
/// where each measurement has exactly one single fractional digit
///
/// ```
/// let measurements = "
///     Hamburg;12.0
///     Bulawayo;8.9
///     Palembang;38.8
///     St. John's;15.2
///     Cracow;12.6
///     Bridgetown;26.9
///     Istanbul;6.2
///     Roseau;34.4
///     Conakry;31.2
///     Istanbul;23.0
/// "
/// ```
///
/// the task is to read the whole file, and:
/// - calculate the min temperature;
/// - calculate the mean temperature;
/// - calculate the max temperature.
///
/// for each weather station, and emit the result in the stdout:
/// sorted alphabetically by station name, and the result values per station in the format `<min>/<mean>/<max>`, rounded to one fractional digit.
///
/// ```
/// let result = "{Abha=-23.0/18.0/59.2, Abidjan=-16.2/26.0/67.3, Abéché=-10.0/29.4/69.0, ...}"
/// ```

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

fn main() {
    let mut statuses = HashMap::<&str, Status>::new();

    let path = current_dir().unwrap().join("measurements.txt");
    let file = File::open(path).unwrap();
    let bytes = mmap(&file);

    let mut index = 0;
    let mut prev = 0;

    for byte in bytes {
        if byte != &NEW_LINE {
            index += 1;
            continue;
        }

        let line = unsafe { str::from_utf8_unchecked(&bytes[prev..index]) };

        let (station, temperature) = line.split_once(";").unwrap();
        let temperature: f64 = temperature.parse().unwrap();

        let status = statuses.entry(station).or_default();

        status.max = temperature.max(status.max);
        status.min = temperature.min(status.min);
        status.sum += temperature;
        status.count += 1;

        prev = index + 1;
        index += 1;
    }

    let mut sorted = statuses.keys().collect::<Vec<_>>();

    sorted.sort_unstable();

    let mut sorted = sorted.into_iter().peekable();

    print!("{{");

    while let Some(station) = sorted.next() {
        let status = statuses.get(station).unwrap();

        print!(
            "{}={:.1}/{:.1}/{:.1}",
            station,
            status.min,
            status.sum / status.count as f64,
            status.max
        );

        if let Some(_) = sorted.peek() {
            print!(", ");
        }
    }

    print!("}}");
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

        if libc::madvise(ptr, len, libc::MADV_HUGEPAGE) != 0 {
            panic!("{:?}", io::Error::last_os_error());
        }

        slice::from_raw_parts(ptr as *const u8, len as usize)
    }
}

// TODO: maybe munmap? it's needed if we're just shutting down the program?
