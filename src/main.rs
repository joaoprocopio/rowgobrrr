#![feature(slice_split_once)]

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::env::{args, current_dir};
use std::fs::File;
use std::io;
use std::io::Write;

use beecrab::core::{Metrics, Temperature, TemperatureSum};
use beecrab::mmap::Mmap;

type MetricsMap<'a> = HashMap<&'a [u8], Metrics>;

const NEW_LINE: u8 = b'\n';
const SEPARATOR: u8 = b';';
// const NEGATIVE: u8 = b'-';
// const DOT: u8 = b'.';

fn main() {
    let mut metrics = MetricsMap::with_capacity(2048);

    let filename = args()
        .nth(1)
        .expect("measurements file path should be provider");

    let file = current_dir()
        .and_then(|path| path.join(filename).canonicalize())
        .and_then(|path| File::open(path))
        .unwrap();

    let slice = Mmap::map(&file).unwrap();

    slice
        .split(|byte| *byte == NEW_LINE)
        .filter(|byte| !byte.is_empty())
        .for_each(|line| {
            let (station, temperature) = line.split_once(|&byte| byte == SEPARATOR).unwrap();
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

    write_results(metrics);
}

fn parse_temperature<'a>(slice: &'a [u8]) -> Temperature {
    unsafe { str::from_utf8_unchecked(slice) }.parse().unwrap()
}

fn write_results(metrics: MetricsMap) {
    let mut sorted = metrics.keys().collect::<Vec<_>>();
    sorted.sort_unstable();

    let mut sorted = sorted.into_iter().peekable();

    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    write!(writer, "{{").unwrap();

    while let Some(station) = sorted.next() {
        let status = metrics.get(station).unwrap();
        let station = unsafe { str::from_utf8_unchecked(station) };

        write!(
            writer,
            "{}={:.1}/{:.1}/{:.1}",
            station,
            status.min,
            status.sum / status.count as TemperatureSum,
            status.max
        )
        .unwrap();

        if let Some(_) = sorted.peek() {
            write!(writer, ", ").unwrap();
        }
    }

    writeln!(writer, "}}").unwrap();

    writer.flush().unwrap();
}
