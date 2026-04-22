use beecrab::metrics::Metrics;
use beecrab::metrics::NEWLINE;
use beecrab::mmap::Mmap;
use libc;
use std::env::{args, current_dir};
use std::fs::File;
use std::io;
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
        libc::MAP_PRIVATE,
        file.as_raw_fd(),
        0,
    )
    .unwrap();

    let threads = thread::available_parallelism()
        .map(|threads| threads.get())
        .unwrap_or(1);

    mmap.advise(if threads > 1 {
        libc::MADV_NORMAL
    } else {
        libc::MADV_SEQUENTIAL
    })
    .unwrap();
    mmap.advise(libc::MADV_HUGEPAGE).unwrap();
    mmap.advise(libc::MADV_WILLNEED).unwrap();

    let buffer = mmap.as_slice();
    let ranges = chunk_ranges(buffer, threads);
    let metrics = compute_ranges(buffer, ranges);

    let writer = io::BufWriter::new(io::stdout().lock());
    metrics.render(writer).unwrap();
}

fn chunk_ranges(buffer: &[u8], chunks: usize) -> Vec<Range<usize>> {
    if buffer.is_empty() {
        return Vec::new();
    }

    if chunks <= 1 {
        return vec![0..buffer.len()];
    }

    let mut boundaries = Vec::with_capacity(chunks + 1);
    boundaries.push(0);

    for index in 1..chunks {
        let target = index * buffer.len() / chunks;
        let boundary = next_record_boundary(buffer, target);

        if boundary > *boundaries.last().unwrap() && boundary < buffer.len() {
            boundaries.push(boundary);
        }
    }

    boundaries.push(buffer.len());

    boundaries
        .windows(2)
        .filter_map(|window| {
            let start = window[0];
            let end = window[1];

            (start < end).then_some(start..end)
        })
        .collect()
}

fn compute_ranges<'a>(buffer: &'a [u8], ranges: Vec<Range<usize>>) -> Metrics<'a> {
    match ranges.len() {
        0 => Metrics::new(),
        1 => {
            let mut metrics = Metrics::new();
            metrics.compute(&buffer[ranges[0].clone()]);
            metrics
        }
        _ => thread::scope(|scope| {
            let handles: Vec<_> = ranges
                .into_iter()
                .map(|range| {
                    let slice = &buffer[range];

                    scope.spawn(move || {
                        let mut metrics = Metrics::new();
                        metrics.compute(slice);
                        metrics
                    })
                })
                .collect();

            let mut metrics = Metrics::new();

            for handle in handles {
                metrics.merge(handle.join().unwrap());
            }

            metrics
        }),
    }
}

fn next_record_boundary(buffer: &[u8], offset: usize) -> usize {
    if offset == 0 {
        return 0;
    }

    buffer[offset..]
        .iter()
        .position(|byte| *byte == NEWLINE)
        .map(|relative| offset + relative + 1)
        .unwrap_or(buffer.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_ranges_align_records() {
        let buffer = b"alpha;1.0\nbeta;2.0\ngamma;3.0\ndelta;4.0\n";

        let ranges = chunk_ranges(buffer, 3);

        assert!(!ranges.is_empty());
        assert_eq!(ranges.first().unwrap().start, 0);
        assert_eq!(ranges.last().unwrap().end, buffer.len());

        for range in ranges {
            assert!(range.start == 0 || buffer[range.start - 1] == NEWLINE);
            assert!(range.end == buffer.len() || buffer[range.end - 1] == NEWLINE);
        }
    }

    #[test]
    fn chunk_ranges_skip_empty_slices() {
        let buffer = b"alpha;1.0\nbeta;2.0\n";

        let ranges = chunk_ranges(buffer, 16);

        assert_eq!(ranges, vec![0..10, 10..19]);
    }

    #[test]
    fn split_ranges_compute_in_parallel() {
        let buffer = b"alpha;1.0\nbeta;2.0\nalpha;3.0\ngamma;-4.5";
        let ranges = chunk_ranges(buffer, 3);
        let metrics = compute_ranges(buffer, ranges);

        let mut result = Vec::new();
        metrics.render(&mut result).unwrap();

        assert_eq!(
            String::from_utf8(result).unwrap(),
            "{alpha=1.0/2.0/3.0, beta=2.0/2.0/2.0, gamma=-4.5/-4.5/-4.5}\n"
        );
    }
}
