#![feature(portable_simd)]

use std::env::{args, current_dir};
use std::fs::File;
use std::io::BufWriter;
use std::io::{Write, stdout};
use std::os::fd::AsRawFd;
use std::simd::cmp::SimdPartialEq;
use std::simd::u8x32;

use beecrab::core::{MetricsMap, TemperatureSum};
use beecrab::mmap::Mmap;

fn main() {
    let filename = args()
        .nth(1)
        .expect("measurements file path should be provider");
    let file = current_dir()
        .and_then(|path| path.join(filename).canonicalize())
        .and_then(|path| File::open(path))
        .unwrap();

    let map = Mmap::new(file.metadata().unwrap().len() as usize, file.as_raw_fd(), 0).unwrap();

    let metrics = compute_metrics(map.as_slice());
    write_metrics(metrics);
}

fn compute_metrics<'a>(buffer: &'a [u8]) -> MetricsMap<'a> {
    let mut metrics = MetricsMap::with_capacity(512);

    let semi = u8x32::splat(b';');
    let newl = u8x32::splat(b'\n');

    // TODO: process the remainder
    let (chunks, _remainder) = buffer.as_chunks::<32>();

    for chunk in chunks {
        let chunk = u8x32::from_slice(chunk);
        let semimsk = chunk.simd_eq(semi);
        let newlmsk = chunk.simd_eq(newl);
    }

    // buffer
    //     .split(|byte| *byte == b'\n')
    //     .filter(|byte| !byte.is_empty())
    //     .for_each(|line| {
    //         let (station, temperature) = line.split_once(|&byte| byte == b';').unwrap();
    //         let temperature = parse_temperature(temperature);

    //         match metrics.entry(station) {
    //             Entry::Vacant(none) => {
    //                 none.insert(Metrics::new(temperature));
    //             }
    //             Entry::Occupied(mut some) => {
    //                 some.get_mut().update(temperature);
    //             }
    //         };
    //     });

    metrics
}

fn write_metrics(metrics: MetricsMap) {
    let mut stations = metrics.keys().collect::<Vec<_>>();
    stations.sort_unstable();
    let mut stations = stations.into_iter().peekable();
    let mut writer = BufWriter::new(stdout().lock());

    write!(writer, "{{").unwrap();

    while let Some(station) = stations.next() {
        let status = metrics.get(station).unwrap();
        let station = unsafe { str::from_utf8_unchecked(station) };

        write!(
            writer,
            "{}={:.1}/{:.1}/{:.1}",
            station,
            status.min as f64 / 10.0,
            (status.sum / status.count as TemperatureSum) as f64 / 10.0,
            status.max as f64 / 10.0
        )
        .unwrap();

        if let Some(_) = stations.peek() {
            write!(writer, ", ").unwrap();
        }
    }

    writeln!(writer, "}}").unwrap();
}
