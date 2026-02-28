use nih_plug::prelude::*;
use vizia_plug::ViziaState;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use nih_plug::prelude::SmoothingStyle::Linear;
use crate::editor;
use crate::metre::combined_metre_data::CombinedMetreData;

#[derive(Params)]
pub struct MetreFiddlerParams {
    /// The editor state, saved together with the parameter state so the custom scaling can be
    /// restored.
    #[persist = "editor-state"]
    pub editor_state: Arc<ViziaState>,

    #[id = "use_bpm"]
    pub use_bpm: BoolParam,

    #[id = "metric_dur_selector"]
    pub metric_dur_selector: FloatParam,

    #[id = "velocity_min"]
    pub velocity_min: FloatParam,
    #[id = "velocity_max"]
    pub velocity_max: FloatParam,

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
    // This holds the value that is displayed when use_position is false
    pub displayed_position: Arc<AtomicF32>,

    #[id = "reset_phase"]
    pub reset_phase: BoolParam,
    // This informs the Gui, that the phase_reset button needs resetting.
    pub reset_info: Arc<AtomicBool>,
    
    // The `Arc<Mutex<>>` allows to share and modify it
    // between the GUI thread and the audio thread safely.
    #[persist = "combined_metre_data"]
    pub combined_metre_data: Arc<Mutex<CombinedMetreData>>,

    // Interpolate between A and B
    #[id = "interpolate_a_b"]
    pub interpolate_a_b: FloatParam,

    #[persist = "current_nr_of_beats"]
    pub current_nr_of_beats: AtomicUsize,

    #[id = "send_midi"]
    pub send_midi: BoolParam,

    #[persist = "interpolate_durations"]
    pub interpolate_durations: AtomicBool,

    #[persist = "many_velocities"]
    pub many_velocities: AtomicBool,

    #[persist = "midi_out_one_note"]
    pub midi_out_one_note: AtomicBool,
    
    #[persist = "interpolate_indisp"]
    pub interpolate_indisp: AtomicBool,
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

            combined_metre_data: Arc::new(Mutex::new(CombinedMetreData::default())),

            interpolate_a_b: FloatParam::new(
                "Interpolate between Metre A and B",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
                .with_smoother(Linear(50.0)),
            
            current_nr_of_beats: AtomicUsize::new(0),

            send_midi: BoolParam::new(
                "Send midi notes instead",
                false,
            ),

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

            velocity_skew: FloatParam::new(
                "Skew value for Velocity Range",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
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
                "Use and automate Position, not Duration",
                false
            ),

            displayed_position: Arc::new(AtomicF32::new(0.0)),

            reset_info: Arc::new(AtomicBool::new(false)),

            interpolate_durations: AtomicBool::from(true),

            many_velocities: AtomicBool::from(true),

            midi_out_one_note: AtomicBool::from(false),
            
            interpolate_indisp: AtomicBool::from(true),
        }
    }
}