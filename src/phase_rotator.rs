const HILBERT_TAPS: usize = 129;
const GROUP_DELAY: usize = (HILBERT_TAPS - 1) / 2;

pub const HILBERT_GROUP_DELAY_SAMPLES: u32 = GROUP_DELAY as u32;

/// A windowed Hilbert transformer that produces a delayed dry signal and its quadrature pair.
pub struct HilbertTransformer {
    samples: [f32; HILBERT_TAPS],
    coefficients: [f32; HILBERT_TAPS],
    write_position: usize,
}

impl HilbertTransformer {
    pub fn new() -> Self {
        let mut coefficients = [0.0; HILBERT_TAPS];
        for (index, coefficient) in coefficients.iter_mut().enumerate() {
            let offset = index as i32 - GROUP_DELAY as i32;
            if offset != 0 && offset % 2 != 0 {
                let position = index as f32 / (HILBERT_TAPS - 1) as f32;
                let window = 0.42 - 0.5 * (std::f32::consts::TAU * position).cos()
                    + 0.08 * (2.0 * std::f32::consts::TAU * position).cos();
                *coefficient = 2.0 * window / (std::f32::consts::PI * offset as f32);
            }
        }

        Self {
            samples: [0.0; HILBERT_TAPS],
            coefficients,
            write_position: 0,
        }
    }

    pub fn reset(&mut self) {
        self.samples.fill(0.0);
        self.write_position = 0;
    }

    pub fn process(&mut self, input: f32) -> (f32, f32) {
        self.samples[self.write_position] = input;

        let mut quadrature = 0.0;
        for (tap, coefficient) in self.coefficients.iter().enumerate() {
            let index = (self.write_position + HILBERT_TAPS - tap) % HILBERT_TAPS;
            quadrature += self.samples[index] * coefficient;
        }

        let dry_index = (self.write_position + HILBERT_TAPS - GROUP_DELAY) % HILBERT_TAPS;
        let delayed_dry = self.samples[dry_index];
        self.write_position = (self.write_position + 1) % HILBERT_TAPS;

        (delayed_dry, quadrature)
    }
}

#[cfg(test)]
mod tests {
    use super::{GROUP_DELAY, HilbertTransformer};

    #[test]
    fn returns_the_dry_signal_at_the_hilbert_group_delay() {
        let mut transformer = HilbertTransformer::new();
        let output: Vec<_> = (0..=GROUP_DELAY)
            .map(|index| transformer.process(if index == 0 { 1.0 } else { 0.0 }).0)
            .collect();

        assert!(output[..GROUP_DELAY].iter().all(|sample| *sample == 0.0));
        assert_eq!(output[GROUP_DELAY], 1.0);
    }
}
