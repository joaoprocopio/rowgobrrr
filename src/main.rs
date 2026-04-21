use beecrab::metrics::Metrics;
use beecrab::mmap::Mmap;
use libc;
use std::env::{args, current_dir};
use std::fs::File;
use std::os::fd::AsRawFd;

fn main() {
    let filename = args()
        .nth(1)
        .expect("measurements file path should be provider");
    let file = current_dir()
        .and_then(|path| path.join(filename).canonicalize())
        .and_then(|path| File::open(path))
        .unwrap();

    // TODO: when the code is running on parallel, flags should be configured
    let mmap = Mmap::new(
        file.metadata().unwrap().len() as usize,
        libc::PROT_READ,
        libc::MAP_PRIVATE,
        file.as_raw_fd(),
        0,
    )
    .unwrap();

    mmap.advise(libc::MADV_SEQUENTIAL).unwrap();
    mmap.advise(libc::MADV_HUGEPAGE).unwrap();
    mmap.advise(libc::MADV_WILLNEED).unwrap();

    let mut metrics = Metrics::new();
    metrics.compute(mmap.as_slice());
    metrics.render().unwrap();
}
