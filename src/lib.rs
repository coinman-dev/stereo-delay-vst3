mod delay_line;

use delay_line::DelayLine;
use nih_plug::prelude::*;
use std::{num::NonZeroU32, sync::Arc};

const MAX_OFFSET_MS: f32 = 50.0;
const MAX_PHYSICAL_DELAY_MS: f32 = MAX_OFFSET_MS * 2.0;

pub struct StereoDelay {
    params: Arc<StereoDelayParams>,
    left_delay: DelayLine,
    right_delay: DelayLine,
    sample_rate: f32,
    reported_latency_samples: u32,
}

#[derive(Params)]
pub struct StereoDelayParams {
    #[id = "left-offset"]
    pub left_offset: FloatParam,

    #[id = "right-offset"]
    pub right_offset: FloatParam,
}

impl Default for StereoDelay {
    fn default() -> Self {
        Self {
            params: Arc::new(StereoDelayParams::default()),
            left_delay: DelayLine::new(2),
            right_delay: DelayLine::new(2),
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

        Self {
            // This must remain unsmoothed: the reported host latency always matches the current
            // channel offsets, including when the controls are automated.
            left_offset: FloatParam::new("Left Offset", 0.0, range)
                .with_step_size(0.1)
                .with_unit(" ms"),
            right_offset: FloatParam::new("Right Offset", 0.0, range)
                .with_step_size(0.1)
                .with_unit(" ms"),
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
        self.reported_latency_samples = self.required_compensation_samples(
            self.params.left_offset.value(),
            self.params.right_offset.value(),
        );
        context.set_latency_samples(self.reported_latency_samples);

        true
    }

    fn reset(&mut self) {
        self.left_delay.reset();
        self.right_delay.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let left_offset = self.params.left_offset.value();
        let right_offset = self.params.right_offset.value();
        let compensation_samples = self.required_compensation_samples(left_offset, right_offset);
        if compensation_samples != self.reported_latency_samples {
            context.set_latency_samples(compensation_samples);
            self.reported_latency_samples = compensation_samples;
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
            *left = self.left_delay.process(*left, left_delay);
            *right = self.right_delay.process(*right, right_delay);
        }

        ProcessStatus::Normal
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
    use super::StereoDelay;

    #[test]
    fn reports_only_the_required_compensation_for_negative_offsets() {
        let plugin = StereoDelay {
            sample_rate: 48_000.0,
            ..Default::default()
        };

        assert_eq!(plugin.required_compensation_samples(0.0, 5.0), 0);
        assert_eq!(plugin.required_compensation_samples(-5.0, 5.0), 240);
        assert_eq!(plugin.required_compensation_samples(-5.0, -10.0), 480);
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
