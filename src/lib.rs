#![feature(portable_simd)]

use crate::metrics::{MetricsMap, Temperature};
use std::io::BufWriter;
use std::io::{Write, stdout};
use std::simd::cmp::SimdPartialEq;

pub mod metrics;
pub mod mmap;

const U8AVXLNS: usize = 64;

type U8AVX = std::simd::Simd<u8, U8AVXLNS>;

const SEMI: U8AVX = U8AVX::splat(b';');
const NEWL: U8AVX = U8AVX::splat(b'\n');

pub fn compute_metrics<'a>(buffer: &'a [u8]) -> MetricsMap<'a> {
    let metrics = MetricsMap::with_capacity(512);

    let mut cursor = 0usize;

    while cursor < buffer.len() {
        let rng = cursor..cursor + U8AVXLNS;

        if rng.end <= buffer.len() {
            let chunk = U8AVX::from_slice(&buffer[rng]);
            let semimask = chunk.simd_eq(SEMI);
            let newlmask = chunk.simd_eq(NEWL);

            println!("{}", unsafe { str::from_utf8_unchecked(&chunk.as_ref()) });
            dbg!(semimask);
            dbg!(newlmask);

            cursor += U8AVXLNS;
        } else {
            // TODO: process scalar
            // process scalar
            cursor += 1;
        }
    }

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
            (status.sum / status.count as Temperature) as f64 / 10.0,
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
