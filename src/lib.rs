use nih_plug::prelude::*;
use vizia_plug::ViziaState;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use nih_plug::prelude::SmoothingStyle::Linear;
use crate::metre_data::{MetreData};
use crate::util::{decider, rescale};

mod editor;
mod metre_data;
mod metre;
mod gui;
mod util;

struct MetreFiddler {
    params: Arc<MetreFiddlerParams>,
    sample_rate: f32,
    last_reset_phase_value: bool,
    metric_duration: f32,
    progress_in_samples: u64,
    vel_min: f32,
    vel_max: f32,
    lower_threshold: f32,
    upper_threshold: f32,
}

#[derive(Params)]
struct MetreFiddlerParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,

    #[id = "use_bpm"]
    pub use_bpm: BoolParam,
    
    #[id = "metric_dur_selector"]
    pub metric_dur_selector: FloatParam,

    #[id = "velocity_min"]
    pub velocity_min: FloatParam,
    #[id = "velocity_max"]
    pub velocity_max: FloatParam,
    
    #[id = "velocity_skew"]
    pub velocity_skew: FloatParam,
    
    #[id = "lower_threshold"]
    pub lower_threshold: FloatParam,
    #[id = "upper_threshold"]
    pub upper_threshold: FloatParam,

    #[id = "bar_position"]
    pub bar_position: FloatParam,
    #[id = "reset_phase"]
    pub reset_phase: BoolParam,
    
    #[id = "use_position"]
    pub use_position: BoolParam,
    
    
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
            last_reset_phase_value: false,
            metric_duration: 1.0,
            progress_in_samples: 0,
            vel_min: 0.0,
            vel_max: 1.0,
            lower_threshold: 0.0,
            upper_threshold: 1.0,
        }
    }
}

impl Default for MetreFiddlerParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            // Select whether to match speed to the DAW's BPM
            use_bpm: BoolParam::new(
                "Use BPM",
                false
            ),
            
            // Select the duration for the metric duration
             metric_dur_selector: FloatParam::new(
                 "Duration Selection",
                 1.0,
                 FloatRange::Skewed{ min: 0.1, max: 20.0, factor: 0.5 },
            )
                 .with_smoother(Linear(50.0)),

            metre_data: Arc::new(Mutex::new(MetreData::default())),

            velocity_min: FloatParam::new(
                "Minimum for the velocity output",
                0.0,
                FloatRange::Linear { min: 0.0, max: 127.0 },
            )
                .with_smoother(Linear(50.0)),
            
            velocity_max: FloatParam::new(
                "Maximum for the velocity output",
                127.0,
                FloatRange::Linear { min: 0.0, max: 127.0 },
            )
                .with_smoother(Linear(50.0)),
            
            velocity_skew: FloatParam::new(
                "Skew value for Velocity Range",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
                .with_smoother(Linear(50.0)),

            lower_threshold: FloatParam::new(
                "Lower Threshold for the Midi output",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0},
            )
                .with_smoother(Linear(50.0)),

            upper_threshold: FloatParam::new(
                "Upper Threshold for the Midi output",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0},
            )
                .with_smoother(Linear(50.0)),

            reset_phase: BoolParam::new(
                "Reset metric phasse",
                false
            ),
            
            bar_position: FloatParam::new(
                "The current position within a bar",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0},
            )
                .with_smoother(Linear(50.0)),
            
            use_position: BoolParam::new(
              "Use and automate the Position within the Bar, instead of the Duration for the bar",
              false
            ),
            
            reset_info: Arc::new(AtomicBool::new(false)),
        }
    }
}

// TODO Logic for velocity skew and bar_position
impl MetreFiddler {
    fn process_event<S: SysExMessage>(&mut self, event: NoteEvent<S>) -> Option<NoteEvent<S>> {
        let metric_data = &self.params.metre_data.lock().unwrap();
        let metric_durations = &metric_data.durations;
        let indisp_ls = &metric_data.value;
        let max = indisp_ls.len() - 1;
        
        // time in seconds
        let time = self.progress_in_samples as f32 / self.sample_rate;
        let time_in_bar_normalized = time.rem_euclid(self.metric_duration) / self.metric_duration;
        
        // calculate the indispensability value
        let indisp_idx: usize = if let Ok(idx) = decider(time_in_bar_normalized, &metric_durations) {
            idx as usize
        } else { 0 };
        let indisp_val = indisp_ls[indisp_idx];
        // velocity in range 0 - 1, rescaled by vel_min and vel_max parameters
        let v_min: f32 = self.vel_min.min(self.vel_max) / 127.0;
        let v_max: f32 = self.vel_max / 127.0;
        let vel: f32 = if v_min == v_max {
            v_min
        } else {
            rescale(1.0 / (indisp_val + 1) as f32, 0.0, 1.0, v_min, v_max, true)
                .unwrap_or(0.8)
        };

        match event {
            NoteEvent::NoteOn {
                timing,
                voice_id,
                channel,
                note,
                ..
            } => { if indisp_val >= (self.lower_threshold.min(self.upper_threshold) * max as f32) as usize 
                && indisp_val <= (self.upper_threshold * max as f32) as usize {                
                Some(NoteEvent::NoteOn {
                    timing,
                    voice_id,
                    channel,
                    note,
                    velocity: vel,
                })} else {
                    None
                }
            },
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
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {        
        let mut current_sample: u32 = 0;
        let buffer_len = buffer.samples();
        let mut last_note_was_let_through = true;
        let mut elapsed_samples: u32 = 0;

        // reset progress when playback stops.
        if !context.transport().playing {
            self.progress_in_samples = 0;
        }
                
        // automated value
        if self.params.reset_phase.value() {
            if ! self.last_reset_phase_value {
                // resetting the progress_in_samples counter:
                self.progress_in_samples = 0;
            }
            // message to gui
            self.params.reset_info.store(false, SeqCst)
        }
        self.last_reset_phase_value = self.params.reset_phase.value();
        
        // handle all incoming events
        while let Some(event) = context.next_event() {
            // samples since last event
            elapsed_samples = event.timing() - current_sample;
            // update progress
            self.progress_in_samples += elapsed_samples as u64;
            current_sample += elapsed_samples;

            // get all parameters for this event
            if elapsed_samples > 0 {
                self.vel_min = self.params.velocity_min.smoothed.next_step(elapsed_samples);
                self.vel_max = self.params.velocity_max.smoothed.next_step(elapsed_samples);
                self.lower_threshold = self.params.lower_threshold.smoothed.next_step(elapsed_samples);
                self.upper_threshold = self.params.upper_threshold.smoothed.next_step(elapsed_samples);
                self.metric_duration =  self.params.metric_dur_selector.smoothed.next_step(elapsed_samples)
            } else {
                self.vel_min = self.params.velocity_min.value();
                self.vel_max = self.params.velocity_max.value();
                self.lower_threshold = self.params.lower_threshold.value();
                self.upper_threshold = self.params.upper_threshold.value();
                self.metric_duration =  self.params.metric_dur_selector.value();
            }
            
            // set duration to length of a quarter note times the slider when bpm toggle is true:
            if self.params.use_bpm.value() {
                let one_crotchet = 60.0 / if let Some(tempo) = context.transport().tempo {
                    tempo
                } else { 60.0 };
                self.metric_duration = one_crotchet as f32 * self.metric_duration;
            };
            
            match event {
                NoteEvent::NoteOn {..} => {
                    if let Some(event) = self.process_event(event) {
                        context.send_event(event);
                        last_note_was_let_through = true;
                    } else {
                        last_note_was_let_through = false;
                    }
                },
                NoteEvent::NoteOff {..} => {
                    if last_note_was_let_through {
                        context.send_event(event)
                    }
                },
                _ => context.send_event(event),
            }
        }
        
        // update progress with samples left in buffer
        elapsed_samples = buffer_len as u32 - current_sample;
        self.progress_in_samples += elapsed_samples as u64;
        // update all parameters once again
        self.params.velocity_min.smoothed.next_step(elapsed_samples);
        self.params.velocity_max.smoothed.next_step(elapsed_samples);
        self.params.lower_threshold.smoothed.next_step(elapsed_samples);
        self.params.upper_threshold.smoothed.next_step(elapsed_samples);
        
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
