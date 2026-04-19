pub type Temperature = f64;
pub type TemperatureSum = i64;
pub type TemperatureCount = usize;

#[derive(Debug)]
pub struct Metrics {
    pub min: Temperature,
    pub max: Temperature,
    pub sum: TemperatureSum,
    pub count: TemperatureCount,
}

impl Metrics {
    pub fn new(temperature: Temperature) -> Self {
        Self {
            max: temperature,
            min: temperature,
            sum: temperature as TemperatureSum,
            count: 1,
        }
    }

    pub fn update(&mut self, temperature: Temperature) {
        self.max = temperature.max(self.max);
        self.min = temperature.min(self.min);
        self.sum += temperature as TemperatureSum;
        self.count += 1;
    }
}
