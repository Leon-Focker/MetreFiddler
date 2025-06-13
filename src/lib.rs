use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{Arc, Mutex};
use crate::metre_data::{MetreData};

mod editor;
mod metre_data;
mod metre;
mod gui;

struct MetreFiddler {
    params: Arc<MetreFiddlerParams>,
    sample_rate: f32,
    samples_since_trigger: usize,
    last_reset_phase_value: bool,
}

#[derive(Params)]
struct MetreFiddlerParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,

    #[id = "bpm_toggle"]
    pub bpm_toggle: BoolParam,
    
    #[id = "metric_dur_selector"]
    pub metric_dur_selector: FloatParam,

    #[id = "velocity_min"]
    pub velocity_min: IntParam,
    #[id = "velocity_max"]
    pub velocity_max: IntParam,

    #[id = "lower_threshold"]
    pub lower_threshold: FloatParam,
    #[id = "upper_threshold"]
    pub upper_threshold: FloatParam,

    #[id = "reset_phase"]
    pub reset_phase: BoolParam,

    // custom data struct, marked with `#[persist]`
    // The `Arc<Mutex<CustomData>>` allows to share and modify it
    // between the GUI thread and the audio thread safely.
    #[persist = "metre_data"]
    pub metre_data: Arc<Mutex<MetreData>>,
}

impl Default for MetreFiddler {
    fn default() -> Self {
        let default_params = Arc::new(MetreFiddlerParams::default());
        Self {
            params: default_params.clone(),
            sample_rate: 1.0,
            samples_since_trigger: 0,
            last_reset_phase_value: false,
        }
    }
}

impl Default for MetreFiddlerParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            // Select whether to match speed to the DAW's BPM
            bpm_toggle: BoolParam::new(
                "BPM Toggle",
                false
            ),
            
            // Select the duration for the metric duration
             metric_dur_selector: FloatParam::new(
                "Duration Selection",
                1.0,
                FloatRange::Linear { min: 0.0, max: 10.0},
            ),

            metre_data: Arc::new(Mutex::new(MetreData::default())),

            velocity_min: IntParam::new(
                "Minimum for the velocity output",
                0,
                IntRange::Linear { min: 0, max: 127 },
            ),
            
            velocity_max: IntParam::new(
                "Maximum for the velocity output",
                127,
                IntRange::Linear { min: 0, max: 127 },
            ),

            lower_threshold: FloatParam::new(
                "Lower Threshold for the Midi output",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0},
            ),

            upper_threshold: FloatParam::new(
                "Upper Threshold for the Midi output",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0},
            ),

            reset_phase: BoolParam::new(
                "Reset metric phasse",
                false
            ),
        }
    }
}

impl MetreFiddler {
    fn trigger_event(&mut self) -> bool {
        let passed_time = self.samples_since_trigger as f32 / self.sample_rate;
        
        if passed_time >= self.params. metric_dur_selector.value() {
            self.samples_since_trigger = 0;
            true
        } else {
            self.samples_since_trigger += 1;
            false }
    }
}

impl Plugin for MetreFiddler {
    const NAME: &'static str = "MetreFiddler";
    const VENDOR: &'static str = "Leon Focker";
    const URL: &'static str = "https://youtu.be/dQw4w9WgXcQ";
    const EMAIL: &'static str = "contact@leonfocker.de";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
    ];
    
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
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
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        
        // TODO change self.params. metric_dur_selector according to bpm_toggle...
        // so that 1 = a quarter note.
        if self.params.bpm_toggle.value() {
            let _bpm = context.transport().tempo;    
        }

        //nih_log!("hihi I'm doing what i should: {:?}", self.params.metre_data.lock().unwrap().value);
        // nih_log!("max: {}", self.params.metre_data.lock().unwrap().max);
                
        for (sample_id, _channel_samples) in buffer.iter_samples().enumerate() {
            if context.transport().playing {
                if self.trigger_event() {
                    context.send_event(NoteEvent::NoteOn {
                        timing: sample_id as u32,
                        voice_id: Some(0),
                        channel: 0,
                        note: 60,
                        velocity: 1.0,
                    });
                    context.send_event(NoteEvent::NoteOn {
                        timing: sample_id as u32,
                        voice_id: Some(0),
                        channel: 0,
                        note: 60,
                        velocity: 1.0,
                    });
                }
            }

            // Reset Phase
            if !self.last_reset_phase_value && self.params.reset_phase.value() {
                // TODO reset phase
                nih_log!("hihi I'm doing what i should: {:?}", self.params.reset_phase.value());
            };
            self.last_reset_phase_value = self.params.reset_phase.value();

        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for MetreFiddler {
    const CLAP_ID: &'static str = "leonfocker.metrefiddler";
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

impl Vst3Plugin for MetreFiddler {
    const VST3_CLASS_ID: [u8; 16] = *b"MetreFiddlerAAaA";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

nih_export_clap!(MetreFiddler);
nih_export_vst3!(MetreFiddler);
