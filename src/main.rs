use beecrab::metrics::Metrics;
use libc;

use std::env::{args, current_dir};
use std::fs::File;
use std::os::fd::AsRawFd;

use beecrab::mmap::Mmap;

fn main() {
    let filename = args()
        .nth(1)
        .expect("measurements file path should be provider");
    let file = current_dir()
        .and_then(|path| path.join(filename).canonicalize())
        .and_then(|path| File::open(path))
        .unwrap();

    // TODO: when the code is running on parallel, flags should be configured
    let map = Mmap::new(
        file.metadata().unwrap().len() as usize,
        libc::PROT_READ,
        libc::MAP_PRIVATE,
        file.as_raw_fd(),
        0,
    )
    .and_then(|map| map.advise(libc::MADV_SEQUENTIAL))
    .and_then(|map| map.advise(libc::MADV_HUGEPAGE))
    .unwrap();

    let mut metrics = Metrics::new();

    metrics.compute(map.as_slice());
    metrics.render().unwrap();
}
