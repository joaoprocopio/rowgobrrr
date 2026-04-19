#![deny(clippy::all)]

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::env::{args, current_dir};
use std::fs::File;
use std::io;
use std::io::Write;

use beecrab::mmap::Mmap;

type Temperature = f64;

#[derive(Debug)]
struct Status {
    min: Temperature,
    max: Temperature,
    sum: Temperature,
    count: usize,
}

impl Status {
    fn new(temperature: Temperature) -> Self {
        Self {
            max: temperature,
            min: temperature,
            sum: temperature,
            count: 1,
        }
    }

    fn update(&mut self, temperature: Temperature) {
        self.max = temperature.max(self.max);
        self.min = temperature.min(self.min);
        self.sum += temperature;
        self.count += 1;
    }
}

const NEW_LINE: u8 = b'\n';
const SEPARATOR: char = ';';

fn main() {
    let mut statuses = HashMap::<&str, Status>::with_capacity(2048);

    let filename = args()
        .nth(1)
        .expect("measurements file path should be provider");

    let file = current_dir()
        .and_then(|path| path.join(filename).canonicalize())
        .and_then(|path| File::open(path))
        .unwrap();

    let map = Mmap::map(&file).unwrap();

    map.split(|byte| *byte == NEW_LINE)
        .filter(|byte| !byte.is_empty())
        .for_each(|line| {
            let line = unsafe { str::from_utf8_unchecked(line) };
            let (station, temperature) = line.split_once(SEPARATOR).unwrap();
            let temperature: Temperature = temperature.parse().unwrap();

            match statuses.entry(station) {
                Entry::Vacant(none) => {
                    none.insert(Status::new(temperature));
                }
                Entry::Occupied(mut some) => {
                    some.get_mut().update(temperature);
                }
            };
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
            status.sum / status.count as Temperature,
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
