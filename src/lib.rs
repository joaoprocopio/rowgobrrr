#![deny(clippy::all)]
#![feature(portable_simd)]

use std::io::BufWriter;
use std::io::{Write, stdout};
use std::simd::u8x32;

use crate::metrics::{MetricsMap, Temperature, TemperatureSum};

pub mod metrics;
pub mod mmap;

pub fn compute_metrics<'a>(buffer: &'a [u8]) -> MetricsMap<'a> {
    let mut metrics = MetricsMap::with_capacity(512);

    let semi = u8x32::splat(b';');
    let newl = u8x32::splat(b'\n');

    // TODO: process the remainder
    let (chunks, _remainder) = buffer.as_chunks::<32>();

    // for chunk in chunks {
    //     let chunk = u8x32::from_slice(chunk);
    //     let semimsk = chunk.simd_eq(semi);
    //     let newlmsk = chunk.simd_eq(newl);
    // }

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

pub fn write_metrics(metrics: MetricsMap) {
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

fn parse_temperature<'a>(buffer: &'a [u8]) -> Temperature {
    let neg = (buffer[0] == b'-') as usize;
    let len = buffer.len();

    // Always valid — dot is at len-2, ones at len-3, frac at len-1
    let frac = (buffer[len - 1] - b'0') as Temperature;
    let ones = (buffer[len - 3] - b'0') as Temperature;

    // tens digit exists only when (len - neg) == 4
    // saturating_sub(4): when len==3, falls back to index 0 (safe, gets masked out)
    let has_tens = (len >= 4 + neg) as Temperature;
    let tens = has_tens * buffer[len.saturating_sub(4)].wrapping_sub(b'0') as Temperature;

    let val = tens * 100 + ones * 10 + frac;

    (1 - 2 * neg as Temperature) * val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_temperature() {
        assert_eq!(parse_temperature(b"0.0"), 0);

        assert_eq!(parse_temperature(b"-9.0"), -90);
        assert_eq!(parse_temperature(b"-9.5"), -95);
        assert_eq!(parse_temperature(b"-9.9"), -99);

        assert_eq!(parse_temperature(b"9.5"), 95);
        assert_eq!(parse_temperature(b"9.9"), 99);
        assert_eq!(parse_temperature(b"9.0"), 90);

        assert_eq!(parse_temperature(b"-99.0"), -990);
        assert_eq!(parse_temperature(b"-99.5"), -995);
        assert_eq!(parse_temperature(b"-99.9"), -999);

        assert_eq!(parse_temperature(b"99.0"), 990);
        assert_eq!(parse_temperature(b"99.5"), 995);
        assert_eq!(parse_temperature(b"99.9"), 999);
    }
}
