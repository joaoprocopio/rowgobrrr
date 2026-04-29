use gxhash::GxBuildHasher;
use std::collections::btree_map::BTreeMap;
use std::collections::hash_map::{Entry, HashMap};
use std::hint;
use std::io;
use std::io::Write;
use std::simd::cmp::SimdPartialEq;

pub const capacity: usize = 2 << 16;

pub const newl: u8 = b'\n';
pub const semi: u8 = b';';

pub const u8x32_semi: u8x32 = u8x32::splat(semi);
pub const u8x32_newl: u8x32 = u8x32::splat(newl);

pub const u8x32_lanes: usize = 32;
pub type u8x32 = std::simd::Simd<u8, u8x32_lanes>;

pub type Temperature = i16;
pub type TemperatureCount = i64;

#[derive(Debug, PartialEq, Eq)]
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
        self.sum += temperature as TemperatureCount;
        self.count += 1;
        if temperature > self.max {
            self.max = temperature
        }
        if temperature < self.min {
            self.min = temperature
        }
    }
}

impl Extend<Aggregate> for Aggregate {
    fn extend<T: IntoIterator<Item = Aggregate>>(&mut self, iter: T) {
        for item in iter {
            self.extend_one(item);
        }
    }

    fn extend_one(&mut self, item: Aggregate) {
        self.sum += item.sum;
        self.count += item.count;
        if item.max > self.max {
            self.max = item.max
        }
        if item.min < self.min {
            self.min = item.min
        }
    }
}

pub struct Metrics<'a> {
    table: HashMap<&'a [u8], Aggregate, GxBuildHasher>,
}

impl<'a> Metrics<'a> {
    pub fn new() -> Self {
        Self {
            table: HashMap::with_capacity_and_hasher(capacity, Default::default()),
        }
    }

    pub fn compute(&mut self, slice: &'a [u8]) {
        let mut cursor = 0;
        let mut line_start_cursor = 0;
        let mut maybe_semicolon_cursor = None;

        while cursor + u8x32_lanes < slice.len() {
            let chunk = u8x32::from_slice(&slice[cursor..cursor + u8x32_lanes]);

            let semicolon_bitmask = chunk.simd_eq(u8x32_semi).to_bitmask();
            let newline_bitmask = chunk.simd_eq(u8x32_newl).to_bitmask();

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

                    self.insert(station, temperature);

                    line_start_cursor = absolute_index + 1;
                    maybe_semicolon_cursor = None;
                }

                bitmask &= bitmask - 1;
            }

            cursor += u8x32_lanes;
        }

        while cursor < slice.len() {
            match slice[cursor] {
                semi => maybe_semicolon_cursor = Some(cursor),
                newl => {
                    let semicolon_cursor = maybe_semicolon_cursor
                        .take()
                        .expect("newline must be before semicolon");

                    let station = &slice[line_start_cursor..semicolon_cursor];
                    let temperature = parse_temperature(&slice[semicolon_cursor + 1..cursor]);

                    self.insert(station, temperature);

                    line_start_cursor = cursor + 1;
                    maybe_semicolon_cursor = None;
                }
                _ => (),
            };

            cursor += 1;
        }
    }

    pub fn render(self, mut writer: impl Write) -> io::Result<()> {
        let mut stations =
            BTreeMap::from_iter(self.table.into_iter().map(|(station, aggregate)| {
                let station = unsafe { str::from_utf8_unchecked(station) };
                let min = aggregate.min as f64 / 10.0;
                let avg = (2 * aggregate.sum + aggregate.count).div_euclid(2 * aggregate.count)
                    as f64
                    / 10.0;
                let max = aggregate.max as f64 / 10.0;

                (station, (min, avg, max))
            }))
            .into_iter()
            .peekable();

        write!(&mut writer, "{{")?;

        while let Some((station, (min, avg, max))) = stations.next() {
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

trait Insert<K, T> {
    fn insert(&mut self, key: K, value: T);
}

impl<'a> Insert<&'a [u8], Temperature> for Metrics<'a> {
    #[inline]
    fn insert(&mut self, key: &'a [u8], value: Temperature) {
        match self.table.entry(key) {
            Entry::Occupied(mut some) => {
                some.get_mut().update(value);
            }
            Entry::Vacant(none) => {
                none.insert(Aggregate::new(value));
            }
        }
    }
}

impl<'a> Insert<&'a [u8], Aggregate> for Metrics<'a> {
    #[inline]
    fn insert(&mut self, key: &'a [u8], value: Aggregate) {
        match self.table.entry(key) {
            Entry::Occupied(mut some) => {
                some.get_mut().extend_one(value);
            }
            Entry::Vacant(none) => {
                none.insert(value);
            }
        }
    }
}

impl<'a> Extend<Metrics<'a>> for Metrics<'a> {
    fn extend<T: IntoIterator<Item = Metrics<'a>>>(&mut self, iter: T) {
        for item in iter {
            self.extend_one(item);
        }
    }

    fn extend_one(&mut self, item: Metrics<'a>) {
        for (station, aggregate) in item.table.into_iter() {
            self.insert(station, aggregate);
        }
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
    use super::*;
    use std::{fs, path::PathBuf};

    fn measure(filename: &str) {
        let input_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("1brc/src/test/resources/samples")
            .join(filename);

        let assert_path = input_path.with_extension("out");

        let input = fs::read(&input_path).unwrap();
        let expected = fs::read(&assert_path).unwrap();

        let mut metrics = Metrics::new();
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
