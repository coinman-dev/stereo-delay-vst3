mod delay_line;
mod phase_rotator;

use delay_line::DelayLine;
use nih_plug::prelude::*;
use phase_rotator::{HILBERT_GROUP_DELAY_SAMPLES, HilbertTransformer};
use std::{num::NonZeroU32, sync::Arc};

const MAX_OFFSET_MS: f32 = 50.0;
const MAX_PHYSICAL_DELAY_MS: f32 = MAX_OFFSET_MS * 2.0;

pub struct StereoDelay {
    params: Arc<StereoDelayParams>,
    left_delay: DelayLine,
    right_delay: DelayLine,
    left_hilbert: HilbertTransformer,
    right_hilbert: HilbertTransformer,
    sample_rate: f32,
    reported_latency_samples: u32,
}

#[derive(Params)]
pub struct StereoDelayParams {
    #[id = "left-offset"]
    pub left_offset: FloatParam,

    #[id = "right-offset"]
    pub right_offset: FloatParam,

    #[id = "left-phase"]
    pub left_phase: FloatParam,

    #[id = "right-phase"]
    pub right_phase: FloatParam,
}

impl Default for StereoDelay {
    fn default() -> Self {
        Self {
            params: Arc::new(StereoDelayParams::default()),
            left_delay: DelayLine::new(2),
            right_delay: DelayLine::new(2),
            left_hilbert: HilbertTransformer::new(),
            right_hilbert: HilbertTransformer::new(),
            sample_rate: 44_100.0,
            reported_latency_samples: 0,
        }
    }
}

impl Default for StereoDelayParams {
    fn default() -> Self {
        let range = FloatRange::Linear {
            min: -MAX_OFFSET_MS,
            max: MAX_OFFSET_MS,
        };
        let phase_range = FloatRange::Linear {
            min: -180.0,
            max: 180.0,
        };

        Self {
            // This must remain unsmoothed: the reported host latency always matches the current
            // channel offsets, including when the controls are automated.
            left_offset: FloatParam::new("Left Offset", 0.0, range)
                .with_step_size(0.1)
                .with_unit(" ms"),
            right_offset: FloatParam::new("Right Offset", 0.0, range)
                .with_step_size(0.1)
                .with_unit(" ms"),
            left_phase: FloatParam::new("Left Phase", 0.0, phase_range)
                .with_step_size(1.0)
                .with_unit(" deg"),
            right_phase: FloatParam::new("Right Phase", 0.0, phase_range)
                .with_step_size(1.0)
                .with_unit(" deg"),
        }
    }
}

impl StereoDelay {
    fn max_delay_samples(sample_rate: f32) -> usize {
        (MAX_PHYSICAL_DELAY_MS * sample_rate / 1_000.0).ceil() as usize
    }

    fn required_compensation_samples(&self, left_offset_ms: f32, right_offset_ms: f32) -> u32 {
        let earliest_offset_ms = left_offset_ms.min(right_offset_ms).min(0.0);
        (-earliest_offset_ms * self.sample_rate / 1_000.0).ceil() as u32
    }

    fn required_latency_samples(
        &self,
        left_offset_ms: f32,
        right_offset_ms: f32,
        phase_rotation_active: bool,
    ) -> u32 {
        let phase_latency = if phase_rotation_active {
            HILBERT_GROUP_DELAY_SAMPLES
        } else {
            0
        };
        phase_latency + self.required_compensation_samples(left_offset_ms, right_offset_ms)
    }

    fn offset_to_delay_samples(&self, offset_ms: f32, compensation_samples: u32) -> f32 {
        (offset_ms * self.sample_rate / 1_000.0 + compensation_samples as f32)
            .clamp(0.0, Self::max_delay_samples(self.sample_rate) as f32)
    }
}

impl Plugin for StereoDelay {
    const NAME: &'static str = "Stereo Delay";
    const VENDOR: &'static str = "Stereo Delay";
    const URL: &'static str = "https://example.invalid/stereo-delay";
    const EMAIL: &'static str = "support@example.invalid";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        let max_delay_samples = Self::max_delay_samples(self.sample_rate);
        self.left_delay = DelayLine::new(max_delay_samples);
        self.right_delay = DelayLine::new(max_delay_samples);
        self.left_hilbert.reset();
        self.right_hilbert.reset();
        self.reported_latency_samples = self.required_latency_samples(
            self.params.left_offset.value(),
            self.params.right_offset.value(),
            self.params.left_phase.value() != 0.0 || self.params.right_phase.value() != 0.0,
        );
        context.set_latency_samples(self.reported_latency_samples);

        true
    }

    fn reset(&mut self) {
        self.left_delay.reset();
        self.right_delay.reset();
        self.left_hilbert.reset();
        self.right_hilbert.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let left_offset = self.params.left_offset.value();
        let right_offset = self.params.right_offset.value();
        let left_phase = self.params.left_phase.value();
        let right_phase = self.params.right_phase.value();
        let phase_rotation_active = left_phase != 0.0 || right_phase != 0.0;
        let compensation_samples = self.required_compensation_samples(left_offset, right_offset);
        let latency_samples =
            self.required_latency_samples(left_offset, right_offset, phase_rotation_active);
        if latency_samples != self.reported_latency_samples {
            context.set_latency_samples(latency_samples);
            self.reported_latency_samples = latency_samples;
        }

        let left_delay = self.offset_to_delay_samples(left_offset, compensation_samples);
        let right_delay = self.offset_to_delay_samples(right_offset, compensation_samples);
        for mut channel_samples in buffer.iter_samples() {
            let mut channels = channel_samples.iter_mut();
            let left = channels
                .next()
                .expect("stereo input layout guarantees left channel");
            let right = channels
                .next()
                .expect("stereo input layout guarantees right channel");
            let (left_dry, left_quadrature) = self.left_hilbert.process(*left);
            let (right_dry, right_quadrature) = self.right_hilbert.process(*right);
            let left_phase_rotated = if phase_rotation_active {
                rotate_phase(left_dry, left_quadrature, left_phase)
            } else {
                *left
            };
            let right_phase_rotated = if phase_rotation_active {
                rotate_phase(right_dry, right_quadrature, right_phase)
            } else {
                *right
            };
            *left = self.left_delay.process(left_phase_rotated, left_delay);
            *right = self.right_delay.process(right_phase_rotated, right_delay);
        }

        ProcessStatus::Normal
    }
}

fn rotate_phase(dry: f32, quadrature: f32, degrees: f32) -> f32 {
    if degrees == 0.0 {
        dry
    } else if degrees.abs() == 180.0 {
        -dry
    } else {
        let radians = degrees.to_radians();
        dry * radians.cos() + quadrature * radians.sin()
    }
}

impl Vst3Plugin for StereoDelay {
    const VST3_CLASS_ID: [u8; 16] = *b"StereoDelayVst3!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_vst3!(StereoDelay);

#[cfg(test)]
mod tests {
    use super::{HILBERT_GROUP_DELAY_SAMPLES, StereoDelay, rotate_phase};

    #[test]
    fn phase_rotation_preserves_zero_and_inverts_at_the_endpoints() {
        assert_eq!(rotate_phase(0.75, 0.25, 0.0), 0.75);
        assert_eq!(rotate_phase(0.75, 0.25, -180.0), -0.75);
        assert_eq!(rotate_phase(0.75, 0.25, 180.0), -0.75);
        assert!((rotate_phase(0.75, 0.25, 90.0) - 0.25).abs() < 1.0e-6);
    }

    #[test]
    fn reports_only_the_required_compensation_for_negative_offsets() {
        let plugin = StereoDelay {
            sample_rate: 48_000.0,
            ..Default::default()
        };

        assert_eq!(plugin.required_latency_samples(0.0, 5.0, false), 0);
        assert_eq!(plugin.required_latency_samples(-5.0, 5.0, false), 240);
        assert_eq!(plugin.required_latency_samples(-5.0, -10.0, false), 480);
        assert_eq!(
            plugin.required_latency_samples(0.0, 0.0, true),
            HILBERT_GROUP_DELAY_SAMPLES
        );
        assert_eq!(
            plugin.required_latency_samples(-5.0, 5.0, true),
            HILBERT_GROUP_DELAY_SAMPLES + 240
        );
    }

    #[test]
    fn offsets_are_relative_to_the_dynamic_host_compensation() {
        let plugin = StereoDelay {
            sample_rate: 48_000.0,
            ..Default::default()
        };
        let compensation_samples = plugin.required_compensation_samples(-5.0, 5.0);

        assert_eq!(
            plugin.offset_to_delay_samples(-5.0, compensation_samples),
            0.0
        );
        assert_eq!(
            plugin.offset_to_delay_samples(5.0, compensation_samples),
            480.0
        );
    }
}
