use nih_plug::prelude::*;
use nih_plug_iced::IcedState;
use std::sync::Arc;

mod editor;

/// This is mostly identical to the gain example, minus some fluff, and with a GUI.
struct BitFiddler {
    params: Arc<BitFiddlerParams>,
}

#[derive(Params)]
struct BitFiddlerParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<IcedState>,

    #[id = "bit_selector"]
    pub bit_selector: IntParam,
}

impl Default for BitFiddler {
    fn default() -> Self {
        let default_params = Arc::new(BitFiddlerParams::default());
        Self {
            params: default_params.clone(),
        }
    }
}

impl Default for BitFiddlerParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            // Select which bit to flip
            bit_selector: IntParam::new(
                "Bit Selection",
                0,
                // Is there a way/necessity to not hardcode the 31?
                IntRange::Linear { min: 0, max: 31},
            )
        }
    }
}

impl Plugin for BitFiddler {
    const NAME: &'static str = "BitFiddler";
    const VENDOR: &'static str = "Leon Focker";
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "contact@leonfocker.de";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.params.editor_state.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for channel_samples in buffer.iter_samples() {

            let bit_selector: usize = self.params.bit_selector.value() as usize;

            for sample in channel_samples {
                // transmute the sample into byte representation
                let mut bytes = sample.to_be_bytes();
                // flip one bit by applying a bitmask and xor
                bytes[bit_selector / 8] ^= 1 << 7 - (bit_selector % 8);
                // transmute back to a float and set new sample
                *sample = f32::from_bits(u32::from_be_bytes(bytes));
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for BitFiddler {
    const CLAP_ID: &'static str = "leonfocker.bitfiddler";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A simple distortion plugin flipping one bit of every sample");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Mono,
        ClapFeature::Utility,
    ];
}

impl Vst3Plugin for BitFiddler {
    const VST3_CLASS_ID: [u8; 16] = *b"BitfiddlerAAaAAa";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(BitFiddler);
nih_export_vst3!(BitFiddler);
