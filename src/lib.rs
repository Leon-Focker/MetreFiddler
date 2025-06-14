use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
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
    metric_duration: f32,
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
    
    pub reset_info: Arc<AtomicBool>,

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
            metric_duration: 1.0,
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
            )
                 .with_smoother(SmoothingStyle::Linear(50.0)),

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
            
            reset_info: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn decider(normalized_posiion: f32, weights: Vec<f32>) -> usize {
    // TODO
    0
}
impl MetreFiddler {
    // Todo
    fn process_event<S: SysExMessage>(&mut self, event: NoteEvent<S>, elapsed_samples: u32) -> Option<NoteEvent<S>> {
        let metric_data = self.params.metre_data.lock().unwrap();
        let durations = metric_data.durations.clone();
        let indisp_ls = metric_data.value.clone();
        let normalized_position = 0.0; // TODO
        let indisp_val = indisp_ls[decider(normalized_position, durations)];

        match event {
            NoteEvent::NoteOn {
                timing,
                voice_id,
                channel,
                note,
                velocity,
            } => Some(NoteEvent::NoteOn {
                timing,
                voice_id,
                channel,
                note,
                velocity,
            }),
            _ => None,
        }

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
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        
        let mut current_sample = 0;
        let mut last_note_was_let_through = true;
                
        // automated value
        if self.params.reset_phase.value() {
            if ! self.last_reset_phase_value {
                // TODO reset phase here:
                nih_log!("hihi I'm doing what i should: {:?}", self.params.reset_phase.value());
            }
            // message to gui
            self.params.reset_info.store(false, SeqCst)
        }
        self.last_reset_phase_value = self.params.reset_phase.value();
        
        // TODO all of this is only done once per buffer right? is that meh?
        // with .smoothed.next_step() one can get smoothed values without iterating over all samples.
        // TODO set self.metric_duration according to bpm toggle:
        self.metric_duration = self.params.metric_dur_selector.value();
        //if self.params.bpm_toggle.value() }
        
        // handle all incoming events
        while let Some(event) = context.next_event() {
            let elapsed_samples = event.timing() - current_sample;
            current_sample += elapsed_samples;

            // TODO get all relevant parameters here.

            match event {
                // NoteEvent::NoteOn {..} => {
                //     if let Some(event) = self.process_event(event, elapsed_samples) {
                //         context.send_event(event);
                //         last_note_was_let_through = true;
                //     } else {
                //         last_note_was_let_through = false;
                //     }
                // },
                NoteEvent::NoteOff {..} => {
                    if last_note_was_let_through {
                        context.send_event(event)
                    }
                },
                _ => context.send_event(event),
            }
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
