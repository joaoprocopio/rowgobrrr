use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::io::{self, BufWriter};
use std::io::{Write, stdout};
use std::simd::cmp::SimdPartialEq;
use std::simd::u8x64;

pub type Temperature = i16;
pub type TemperatureCount = i64;

#[derive(Debug)]
pub struct Aggregate {
    pub min: Temperature,
    pub max: Temperature,
    pub sum: Temperature,
    pub count: TemperatureCount,
}

impl Aggregate {
    pub fn new(temperature: Temperature) -> Self {
        Self {
            max: temperature,
            min: temperature,
            sum: temperature,
            count: 1,
        }
    }

    pub fn update(&mut self, temperature: Temperature) {
        self.max = temperature.max(self.max);
        self.min = temperature.min(self.min);
        self.sum += temperature;
        self.count += 1;
    }
}

const LANES: usize = 64;
const SEMI: u8x64 = u8x64::splat(b';');
const NEWL: u8x64 = u8x64::splat(b'\n');

pub struct Metrics<'a> {
    aggregates: HashMap<&'a [u8], Aggregate>,
}

impl<'a> Metrics<'a> {
    pub fn new() -> Self {
        Self {
            aggregates: HashMap::with_capacity(512),
        }
    }

    pub fn compute(&mut self, buffer: &'a [u8]) {
        let (chunks, _remainder) = buffer.as_chunks::<LANES>();

        let mut line_start = 0;
        let mut semi_pos = None;

        for (index, chunk) in chunks.iter().enumerate() {
            let cursor = index * LANES;
            let chunk = u8x64::from_array(*chunk);

            let semi_bitmask = chunk.simd_eq(SEMI).to_bitmask();
            let newl_bitmask = chunk.simd_eq(NEWL).to_bitmask();

            let mut bitmask = semi_bitmask | newl_bitmask;

            while bitmask != 0 {
                let rel = bitmask.trailing_zeros() as usize;
                let abs = cursor + rel;

                if ((semi_bitmask >> rel) & 1) != 0 {
                    semi_pos = Some(abs);
                } else {
                    let semi = semi_pos.take().expect("newline before semicolon");
                    let station = &buffer[line_start..semi];
                    let temperature = parse_temperature(&buffer[semi + 1..abs]);

                    match self.aggregates.entry(station) {
                        Entry::Vacant(none) => {
                            none.insert(Aggregate::new(temperature));
                        }
                        Entry::Occupied(mut some) => {
                            some.get_mut().update(temperature);
                        }
                    }

                    line_start = abs + 1;
                    semi_pos = None;
                }

                bitmask &= bitmask - 1;
            }
        }
    }

    pub fn render(&self) -> io::Result<()> {
        let mut stations = self.aggregates.keys().collect::<Vec<_>>();

        stations.sort_unstable();

        let mut stations = stations.into_iter().peekable();
        let mut writer = BufWriter::new(stdout().lock());

        write!(writer, "{{")?;

        while let Some(station) = stations.next() {
            let status = self.aggregates.get(station).expect("invalid memory state");
            let station = unsafe { str::from_utf8_unchecked(station) };

            write!(
                writer,
                "{}={:.1}/{:.1}/{:.1}",
                station,
                status.min as f64 / 10.0,
                (status.sum / status.count as Temperature) as f64 / 10.0,
                status.max as f64 / 10.0
            )?;

            if let Some(_) = stations.peek() {
                write!(writer, ", ")?;
            }
        }

        writeln!(writer, "}}")?;

        writer.flush()?;

        Ok(())
    }
}

fn parse_temperature(buffer: &[u8]) -> Temperature {
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
