use nice_plug::prelude::*;
use nice_plug::prelude::SmoothingStyle::Linear;
use vizia_plug::ViziaState;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::atomic::Ordering::Relaxed;
use atomic_float::AtomicF32;
use crate::editor;
use crate::metre::combined_metre_data::CombinedMetreData;

// TODO new potential settings: Note Out Duration (how would you input this duration? or just different options like 'short', 'fill_beat'...?),
// TODO         -"-             Note Out base pitch and channel (dropdown menus? :))


#[derive(Params)]
pub struct MetreFiddlerParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    // The `Arc<Mutex<>>` allows to share and modify it
    // between the GUI thread and the audio thread safely.
    #[persist = "combined_metre_data"]
    pub combined_metre_data: Arc<Mutex<CombinedMetreData>>,
    
    // Automatiable Parameters
    
    #[id = "use_bpm"]
    pub use_bpm: BoolParam,

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
    
    #[id = "velocity_skew"]
    pub velocity_skew: FloatParam,

    #[id = "bar_position"]
    pub bar_position: FloatParam,
    
    #[id = "use_position"]
    pub use_position: BoolParam,

    #[id = "reset_phase"]
    pub reset_phase: BoolParam,

    // Interpolate between A and B
    #[id = "interpolate_a_b"]
    pub interpolate_a_b: FloatParam,

    #[id = "send_midi"]
    pub send_midi: BoolParam,
    
    // Simple Parameters
    
    // This informs the Gui, that the phase_reset button needs resetting.
    pub reset_info: AtomicBool,
    
    // This holds the value that is displayed when use_position is false
    pub displayed_position: AtomicF32,
    
    #[persist = "current_nr_of_beats"]
    pub current_nr_of_beats: AtomicUsize,

    #[persist = "interpolate_durations"]
    pub interpolate_durations: AtomicBool,

    #[persist = "many_velocities"]
    pub many_velocities: AtomicBool,

    #[persist = "midi_out_one_note"]
    pub midi_out_one_note: AtomicBool,

    #[persist = "interpolate_indisp"]
    pub interpolate_indisp: AtomicBool,

    #[persist = "retain_metric_phase"]
    pub retain_metric_phase: AtomicBool,
}

impl Default for MetreFiddlerParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            combined_metre_data: Arc::new(Mutex::new(CombinedMetreData::default())),
            
            // Automatable Parameters

            // Select whether to match speed to the DAW's BPM
            use_bpm: BoolParam::new(
                "Use BPM",
                false
            ),
            
            metric_dur_selector: FloatParam::new(
                "Duration Selection",
                1.0,
                FloatRange::Skewed{ min: 0.1, max: 20.0, factor: 0.5 },
            )
                .with_smoother(Linear(50.0)),

            interpolate_a_b: FloatParam::new(
                "Interpolate between Metre A and B",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
                .with_smoother(Linear(50.0)),

            send_midi: BoolParam::new(
                "Send MIDI notes",
                false,
            ),

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

            velocity_skew: FloatParam::new(
                "Skew value for Velocity Range",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),

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
                "Use and automate Position, not Duration",
                false
            ),
            
            // Simple Parameters

            current_nr_of_beats: AtomicUsize::new(0),
            displayed_position: AtomicF32::new(0.0),
            reset_info: AtomicBool::new(false),

            interpolate_durations: AtomicBool::new(true),
            many_velocities: AtomicBool::new(true),
            midi_out_one_note: AtomicBool::new(false),
            interpolate_indisp: AtomicBool::new(true),
            retain_metric_phase: AtomicBool::new(true),
        }
    }
}

impl MetreFiddlerParams {
    /// Return all plain values of Parameters in a ParamsSnapShot,
    /// Parameters that need smoothing will get that somewhere else.
    pub fn snapshot(&self) -> ParamsSnapShot {
        ParamsSnapShot {
            vel_min: self.velocity_min.value() as f32,
            vel_max: self.velocity_max.value() as f32,
            vel_skew: self.velocity_skew.value(),
            lower_threshold: self.lower_threshold.value(),
            upper_threshold: self.upper_threshold.value(),
            bar_pos: self.bar_position.value(),
            interpolate: self.interpolate_a_b.value(),
            use_bpm: self.use_bpm.value(),
            interpolate_durations: self.interpolate_durations.load(Relaxed),
            many_velocities: self.many_velocities.load(Relaxed),
            midi_out_one_note: self.midi_out_one_note.load(Relaxed),
            interpolate_indisp: self.interpolate_indisp.load(Relaxed),
            retain_metric_phase: self.retain_metric_phase.load(Relaxed),
        }
    }
}

pub struct ParamsSnapShot {
    pub vel_min: f32,
    pub vel_max: f32,
    pub vel_skew: f32,
    pub lower_threshold: f32,
    pub upper_threshold: f32,
    pub bar_pos: f32,
    pub interpolate: f32,
    pub use_bpm: bool,
    pub interpolate_durations: bool,
    pub many_velocities: bool,
    pub midi_out_one_note: bool,
    pub interpolate_indisp: bool,
    pub retain_metric_phase: bool,
}

impl Default for ParamsSnapShot {
    fn default() -> Self {
        Self {
            vel_min: 0.0,
            vel_max: 1.0,
            vel_skew: 0.5,
            lower_threshold: 0.0,
            upper_threshold: 1.0,
            bar_pos: 0.0,
            interpolate: 0.0,
            use_bpm: false,
            interpolate_durations: true,
            many_velocities: true,
            midi_out_one_note: false,
            interpolate_indisp: true,
            retain_metric_phase: true,
        }
    }
}