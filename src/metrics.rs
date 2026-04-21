use dashmap::DashMap as ConcurrentHashMap;
use rapidhash::fast::RandomState as FastHasher;
use std::collections::BTreeMap;
use std::hint;
use std::io;
use std::io::Write;
use std::simd::cmp::SimdPartialEq;
use std::simd::u8x64;
use std::sync::Arc;

pub const NEWLINE: u8 = b'\n';
pub const SEMICOLON: u8 = b';';

pub const SEMICOLON_SIMD: u8x64 = u8x64::splat(SEMICOLON);
pub const NEWLINE_SIMD: u8x64 = u8x64::splat(NEWLINE);

pub const SIMD_LANES: usize = 64;

pub type Temperature = i16;
pub type TemperatureCount = i64;

#[derive(Debug)]
pub struct Aggregate {
    pub min: Temperature,
    pub max: Temperature,
    pub sum: TemperatureCount,
    pub count: TemperatureCount,
}

impl Aggregate {
    pub fn new(temperature: Temperature) -> Self {
        Self {
            max: temperature,
            min: temperature,
            sum: temperature as TemperatureCount,
            count: 1,
        }
    }

    pub fn update(&mut self, temperature: Temperature) {
        self.max = temperature.max(self.max);
        self.min = temperature.min(self.min);
        self.sum += temperature as TemperatureCount;
        self.count += 1;
    }
}

type MetricsInner<'a> = ConcurrentHashMap<&'a [u8], Aggregate, FastHasher>;

pub struct Metrics<'a> {
    metrics: Arc<MetricsInner<'a>>,
}

impl<'a> Metrics<'a> {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(MetricsInner::with_capacity_and_hasher_and_shard_amount(
                512,
                FastHasher::new(),
                64,
            )),
        }
    }

    #[inline]
    pub fn upsert(&self, station: &'a [u8], temperature: Temperature) {
        let metrics = self.metrics.clone();

        metrics
            .entry(station)
            .and_modify(|aggregate| {
                aggregate.update(temperature);
            })
            .or_insert_with(|| Aggregate::new(temperature));
    }

    pub fn compute(&self, slice: &'a [u8]) {
        let mut cursor = 0;
        let mut line_start_cursor = 0;
        let mut maybe_semicolon_cursor = None;

        while cursor + SIMD_LANES < slice.len() {
            let chunk = u8x64::from_slice(&slice[cursor..cursor + SIMD_LANES]);

            let semicolon_bitmask = chunk.simd_eq(SEMICOLON_SIMD).to_bitmask();
            let newline_bitmask = chunk.simd_eq(NEWLINE_SIMD).to_bitmask();

            let mut bitmask = semicolon_bitmask | newline_bitmask;

            while bitmask != 0 {
                let relative_index = bitmask.trailing_zeros() as usize;
                let absolute_index = cursor + relative_index;

                if ((semicolon_bitmask >> relative_index) & 1) != 0 {
                    maybe_semicolon_cursor = Some(absolute_index);
                } else {
                    let semicolon_cursor = maybe_semicolon_cursor
                        .take()
                        .expect("newline must be before semicolon");

                    let station = &slice[line_start_cursor..semicolon_cursor];
                    let temperature =
                        parse_temperature(&slice[semicolon_cursor + 1..absolute_index]);

                    self.upsert(station, temperature);

                    line_start_cursor = absolute_index + 1;
                    maybe_semicolon_cursor = None;
                }

                bitmask &= bitmask - 1;
            }

            cursor += SIMD_LANES;
        }

        while cursor < slice.len() {
            match slice[cursor] {
                SEMICOLON => maybe_semicolon_cursor = Some(cursor),
                NEWLINE => {
                    let semicolon_cursor = maybe_semicolon_cursor
                        .take()
                        .expect("newline must be before semicolon");

                    let station = &slice[line_start_cursor..semicolon_cursor];
                    let temperature = parse_temperature(&slice[semicolon_cursor + 1..cursor]);

                    self.upsert(station, temperature);

                    line_start_cursor = cursor + 1;
                    maybe_semicolon_cursor = None;
                }
                _ => (),
            };

            cursor += 1;
        }
    }

    pub fn render(self, mut writer: impl Write) -> io::Result<()> {
        let metrics = Arc::try_unwrap(self.metrics).expect("other Arc references are still alive");
        let mut stations = BTreeMap::from_iter(metrics.into_iter())
            .into_iter()
            .peekable();

        write!(&mut writer, "{{")?;

        while let Some((station, aggregate)) = stations.next() {
            let station = unsafe { str::from_utf8_unchecked(station) };

            let min = aggregate.min as f64 / 10.0;
            let avg =
                (2 * aggregate.sum + aggregate.count).div_euclid(2 * aggregate.count) as f64 / 10.0;
            let max = aggregate.max as f64 / 10.0;

            write!(&mut writer, "{}={:.1}/{:.1}/{:.1}", station, min, avg, max)?;

            if stations.peek().is_some() {
                write!(&mut writer, ", ")?;
            }
        }

        writeln!(&mut writer, "}}")?;

        writer.flush()?;

        Ok(())
    }
}

#[inline]
fn parse_temperature(slice: &[u8]) -> Temperature {
    let len = slice.len();

    unsafe { hint::assert_unchecked(len >= 3) };

    let neg = hint::select_unpredictable(slice[0] == b'-', true, false) as usize;

    // always valid; dot is at len-2, ones at len-3, frac at len-1
    let frac = (slice[len - 1] - b'0') as Temperature;
    let ones = (slice[len - 3] - b'0') as Temperature;

    // tens digit exists only when (len - neg) == 4
    // saturating_sub(4): when len==3, falls back to index 0 (safe, gets masked out)
    let has_tens = hint::select_unpredictable(len >= 4 + neg, true, false) as Temperature;
    let tens = has_tens * slice[len.saturating_sub(4)].wrapping_sub(b'0') as Temperature;

    let val = tens * 100 + ones * 10 + frac;

    (1 - 2 * neg as Temperature) * val
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::*;

    fn measure(filename: &str) {
        let input_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("1brc/src/test/resources/samples")
            .join(filename);

        let assert_path = input_path.with_extension("out");

        let input = fs::read(&input_path).unwrap();
        let expected = fs::read(&assert_path).unwrap();

        let metrics = Metrics::new();
        metrics.compute(&input);

        let mut result = Vec::new();
        metrics.render(&mut result).unwrap();

        assert_eq!(String::from_utf8(result), String::from_utf8(expected))
    }

    #[test]
    fn measurements_1() {
        measure("measurements-1.txt");
    }

    #[test]
    fn measurements_2() {
        measure("measurements-2.txt");
    }

    #[test]
    fn measurements_3() {
        measure("measurements-3.txt");
    }

    #[test]
    fn measurements_10() {
        measure("measurements-10.txt");
    }

    #[test]
    fn measurements_20() {
        measure("measurements-20.txt");
    }

    #[test]
    fn measurements_10000_unique_keys() {
        measure("measurements-10000-unique-keys.txt");
    }

    #[test]
    fn measurements_boundaries() {
        measure("measurements-boundaries.txt");
    }

    #[test]
    fn measurements_complex_utf8() {
        measure("measurements-complex-utf8.txt");
    }

    #[test]
    fn measurements_dot() {
        measure("measurements-dot.txt");
    }

    #[test]
    fn measurements_rounding() {
        measure("measurements-rounding.txt");
    }

    #[test]
    fn measurements_short() {
        measure("measurements-short.txt");
    }

    #[test]
    fn measurements_shortest() {
        measure("measurements-shortest.txt");
    }

    #[test]
    fn parse_temperature_range() {
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
