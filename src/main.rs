#![deny(clippy::all)]

use std::collections::BTreeMap;
use std::env::current_dir;
use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;

/// each line inside the measurements.txt file is in the following format `<string: station_name>;<f64: measurement>`
///
/// where each measurement has exactly one single fractional digit
///
/// ```
/// let measurements = "
///     Hamburg;12.0
///     Bulawayo;8.9
///     Palembang;38.8
///     St. John's;15.2
///     Cracow;12.6
///     Bridgetown;26.9
///     Istanbul;6.2
///     Roseau;34.4
///     Conakry;31.2
///     Istanbul;23.0
/// "
/// ```
///
/// the task is to read the whole file, and:
/// - calculate the min temperature;
/// - calculate the mean temperature;
/// - calculate the max temperature.
///
/// for each weather station, and emit the result in the stdout:
/// sorted alphabetically by station name, and the result values per station in the format `<min>/<mean>/<max>`, rounded to one fractional digit.
///
/// ```
/// let result = "{Abha=-23.0/18.0/59.2, Abidjan=-16.2/26.0/67.3, Abéché=-10.0/29.4/69.0, ...}"
/// ```

type Station = String;
type Temperature = f64;

#[derive(Debug)]
struct Status {
    min: Temperature,
    max: Temperature,
    sum: Temperature,
    count: usize,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            min: Temperature::MAX,
            max: Temperature::MIN,
            sum: 0.,
            count: 0,
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut stats = BTreeMap::<Station, Status>::new();

    let reader = current_dir()
        .and_then(|dir| Ok(dir.join("data/measurements.txt")))
        .and_then(|dir| File::open(dir))
        .and_then(|file| Ok(BufReader::new(file)))?;

    let mut lines = reader.lines();

    while let Some(Ok(line)) = lines.next() {
        let (station, temperature) = line.split_once(";").unwrap();
        let station: Station = station.into();
        let temperature: Temperature = temperature.parse()?;

        let stat = stats.entry(station).or_insert(Status::default());

        stat.max = temperature.max(stat.max);
        stat.min = temperature.min(stat.min);
        stat.sum += temperature;
        stat.count += 1;
    }

    let mut stats = stats.into_iter().peekable();

    print!("{{");

    while let Some((station, status)) = stats.next() {
        print!(
            "{}={:.1}/{:.1}/{:.1}",
            station,
            status.min,
            status.sum / status.count as f64,
            status.max
        );

        if let Some(_) = stats.peek() {
            print!(", ");
        }
    }

    print!("}}");

    Ok(())
}
