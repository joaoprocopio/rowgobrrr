// #![feature(portable_simd)]
#![feature(slice_split_once)]

use std::collections::hash_map::Entry;
use std::env::{args, current_dir};
use std::fs::File;
use std::io;
use std::io::Write;

use beecrab::core::{Metrics, MetricsMap, TemperatureSum, parse_temperature};
use beecrab::mmap::Mmap;

fn main() {
    let filename = args()
        .nth(1)
        .expect("measurements file path should be provider");
    let file = current_dir()
        .and_then(|path| path.join(filename).canonicalize())
        .and_then(|path| File::open(path))
        .unwrap();
    let buffer = Mmap::map(&file).unwrap();

    let metrics = compute_metrics(buffer);
    write_metrics(metrics);
}

fn compute_metrics<'a>(buffer: &'a [u8]) -> MetricsMap<'a> {
    let mut metrics = MetricsMap::with_capacity(256);

    buffer
        .split(|byte| *byte == b'\n')
        .filter(|byte| !byte.is_empty())
        .for_each(|line| {
            let (station, temperature) = line.split_once(|&byte| byte == b';').unwrap();
            let temperature = parse_temperature(temperature);

            match metrics.entry(station) {
                Entry::Vacant(none) => {
                    none.insert(Metrics::new(temperature));
                }
                Entry::Occupied(mut some) => {
                    some.get_mut().update(temperature);
                }
            };
        });

    metrics
}

fn write_metrics(metrics: MetricsMap) {
    let mut stations = metrics.keys().collect::<Vec<_>>();
    stations.sort_unstable();
    let mut stations = stations.into_iter().peekable();
    let mut writer = io::BufWriter::new(io::stdout().lock());

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
