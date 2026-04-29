use beecrab::metrics::Metrics;
use beecrab::metrics::newl;
use beecrab::mmap::Mmap;
use libc;
use std::env::{args, current_dir};
use std::fs::File;
use std::io;
use std::mem;
use std::ops::Range;
use std::os::fd::AsRawFd;
use std::thread;

fn main() {
    let filename = args()
        .nth(1)
        .expect("measurements file path should be provider");
    let file = current_dir()
        .and_then(|path| path.join(filename).canonicalize())
        .and_then(|path| File::open(path))
        .unwrap();

    let mmap = Mmap::new(
        file.metadata().unwrap().len() as usize,
        libc::PROT_READ,
        libc::MAP_SHARED,
        file.as_raw_fd(),
        0,
    )
    .unwrap();

    mmap.advise(libc::MADV_RANDOM).unwrap();
    mmap.advise(libc::MADV_HUGEPAGE).unwrap();
    mmap.advise(libc::MADV_WILLNEED).unwrap();

    let metrics = thread::scope(|scope| {
        let buffer = mmap.as_slice();

        let threads = thread::available_parallelism()
            .map(|threads| threads.get())
            .unwrap_or(1);

        let handles: Vec<_> = chunks(buffer, threads)
            .into_iter()
            .map(|range| {
                scope.spawn(move || {
                    let mut metrics = Metrics::new();
                    metrics.compute(&buffer[range]);
                    metrics
                })
            })
            .collect();

        let mut metrics = Metrics::new();
        metrics.extend(handles.into_iter().map(|handle| handle.join().unwrap()));

        metrics
    });

    let writer = io::BufWriter::new(io::stdout().lock());
    metrics.render(writer).unwrap();

    mem::forget(mmap);
}

fn chunks(buffer: &[u8], count: usize) -> Vec<Range<usize>> {
    if buffer.is_empty() {
        return Vec::new();
    }

    if count <= 1 {
        return vec![0..buffer.len()];
    }

    let mut boundaries: Vec<usize> = Vec::with_capacity(count + 1);

    boundaries.push(0);

    for index in 1..count {
        let target = index * buffer.len() / count;
        let boundary = buffer[target..]
            .iter()
            .position(|byte| *byte == newl)
            .map(|relative| target + relative + 1)
            .unwrap_or(buffer.len());

        if boundary > *boundaries.last().unwrap() && boundary < buffer.len() {
            boundaries.push(boundary);
        }
    }

    boundaries.push(buffer.len());

    let ranges: Vec<Range<usize>> = boundaries
        .windows(2)
        .filter_map(|window| {
            let start = window[0];
            let end = window[1];

            (start < end).then_some(start..end)
        })
        .collect();

    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_ranges_align_records() {
        let buffer = b"alpha;1.0\nbeta;2.0\ngamma;3.0\ndelta;4.0\n";

        let ranges = chunks(buffer, 3);

        assert!(!ranges.is_empty());
        assert_eq!(ranges.first().unwrap().start, 0);
        assert_eq!(ranges.last().unwrap().end, buffer.len());

        for range in ranges {
            assert!(range.start == 0 || buffer[range.start - 1] == newl);
            assert!(range.end == buffer.len() || buffer[range.end - 1] == newl);
        }
    }

    #[test]
    fn chunk_ranges_skip_empty_slices() {
        let buffer = b"alpha;1.0\nbeta;2.0\n";

        let ranges = chunks(buffer, 16);

        assert_eq!(ranges, vec![0..10, 10..19]);
    }
}
