const SAMPLE_COUNT: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MovingAverage {
    window: [u16; SAMPLE_COUNT],
    previous_average: u16,
    position: u8,
}

impl MovingAverage {
    pub fn latest_raw_value(&self) -> u16 {
        self.window[self.position as usize]
    }
    pub fn latest_filtered_value(&self) -> u16 {
        self.previous_average
    }
    pub fn window(&self) -> &[u16] {
        &self.window
    }
    // TODO: Implement efficient recursive average.
    pub fn process(&mut self, value: u16) -> u16 {
        self.window[self.position as usize] = value;
        self.advance();
        let sum: u32 = self.window.iter()
            .map(|v| *v as u32)
            .sum();
        let average = (sum / (SAMPLE_COUNT as u32)) as u16;
        self.previous_average = average;
        average
    }
    fn advance(&mut self) {
        self.position += 1;
        if self.position >= SAMPLE_COUNT as u8 {
            self.position = 0;
        }
    }
}

impl Default for MovingAverage {
    fn default() -> Self {
        Self {
            window: [Default::default(); SAMPLE_COUNT],
            previous_average: Default::default(),
            position: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Max(u16);

impl Max {
    pub fn process(&mut self, value: u16) -> u16 {
        let max = if value > self.0 {
            value
        } else {
            self.0
        };
        self.0 = max;
        max
    }
    pub fn current(&self) -> u16 {
        self.0
    }
}

impl Default for Max {
    fn default() -> Self {
        Self(Default::default())
    }
}
