/// A fractional-sample delay line that never allocates while processing audio.
pub struct DelayLine {
    samples: Vec<f32>,
    write_position: usize,
}

impl DelayLine {
    pub fn new(max_delay_samples: usize) -> Self {
        // Two additional samples guarantee that interpolation is valid at the maximum delay.
        Self {
            samples: vec![0.0; max_delay_samples + 2],
            write_position: 0,
        }
    }

    pub fn reset(&mut self) {
        self.samples.fill(0.0);
        self.write_position = 0;
    }

    pub fn process(&mut self, input: f32, delay_samples: f32) -> f32 {
        self.samples[self.write_position] = input;

        let max_delay = (self.samples.len() - 2) as f32;
        let delay_samples = delay_samples.clamp(0.0, max_delay);
        let read_position =
            (self.write_position as f32 - delay_samples).rem_euclid(self.samples.len() as f32);
        let first_index = read_position.floor() as usize;
        let second_index = (first_index + 1) % self.samples.len();
        let fraction = read_position - first_index as f32;
        let output =
            self.samples[first_index] * (1.0 - fraction) + self.samples[second_index] * fraction;

        self.write_position = (self.write_position + 1) % self.samples.len();
        output
    }
}

#[cfg(test)]
mod tests {
    use super::DelayLine;

    #[test]
    fn delays_an_impulse_by_an_integer_number_of_samples() {
        let mut delay = DelayLine::new(8);
        let output: Vec<_> = [1.0, 0.0, 0.0, 0.0]
            .into_iter()
            .map(|sample| delay.process(sample, 2.0))
            .collect();

        assert_eq!(output, [0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn interpolates_fractional_sample_delays() {
        let mut delay = DelayLine::new(8);
        let output: Vec<_> = [1.0, 0.0, 0.0]
            .into_iter()
            .map(|sample| delay.process(sample, 0.5))
            .collect();

        assert_eq!(output, [0.5, 0.5, 0.0]);
    }

    #[test]
    fn zero_delay_returns_the_current_sample() {
        let mut delay = DelayLine::new(8);

        assert_eq!(delay.process(0.75, 0.0), 0.75);
    }
}
