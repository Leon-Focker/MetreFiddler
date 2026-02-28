use nih_plug::prelude::*;
use std::sync::{Arc};
use std::sync::atomic::Ordering::SeqCst;
use crate::metre::beat_origin::BeatOrigin;
use crate::metre::beat_origin::BeatOrigin::*;
use crate::params::MetreFiddlerParams;
use crate::util::{dry_wet, rescale};

mod editor;
mod metre;
mod gui;
mod util;
mod params;

struct MetreFiddler {
    params: Arc<MetreFiddlerParams>,
    sample_rate: f32,
    progress_in_samples: u64, // TODO maybe this should be reset after each full measure??
    last_reset_phase_value: bool,
    last_sent_beat_idx: i32,
    note_off_buffer: Vec<(u8, i64)>,
    was_playing: bool,

    // TODO these should not be necessary, because they are just parameters:
    vel_min: f32,
    vel_max: f32,
    vel_skew: f32,
    lower_threshold: f32,
    upper_threshold: f32,
    metric_duration: f32,
    bar_pos: f32,
    interpolate: f32,
    interpolate_durs: bool,
    interpolate_indisp: bool,
}

impl Default for MetreFiddler {
    fn default() -> Self {
        let default_params = Arc::new(MetreFiddlerParams::default());
        Self {
            params: default_params.clone(),
            sample_rate: 1.0,
            last_reset_phase_value: false,
            last_sent_beat_idx: -1,
            note_off_buffer: vec![(0, -1), (0, -1), (0, -1), (0, -1)],
            was_playing: false,
            metric_duration: 1.0,
            progress_in_samples: 0,
            vel_min: 0.0,
            vel_max: 1.0,
            vel_skew: 0.5,
            lower_threshold: 0.0,
            upper_threshold: 1.0,
            bar_pos: 0.0,
            interpolate: 0.0,
            interpolate_durs: true,
            interpolate_indisp: true,
        }
    }
}


impl MetreFiddler {

    fn maybe_reset_progress(&mut self, is_playing: bool) {
        if !is_playing && self.was_playing {
            self.progress_in_samples = 0;
            self.was_playing = false;
        } else if is_playing && !self.was_playing {
            self.was_playing = true;
            self.last_sent_beat_idx = -1;
        }
    }

    fn is_indisp_val_within_thresholds(&self, indisp_val: usize, max_indisp_val: usize) -> bool {
        indisp_val >= (self.lower_threshold.min(self.upper_threshold) * max_indisp_val as f32) as usize
            && indisp_val <= (self.upper_threshold * max_indisp_val as f32) as usize
    }

    fn update_velocity_parameters(&mut self) {
        self.vel_min = self.params.velocity_min.value();
        self.vel_max = self.params.velocity_max.value();
        self.vel_skew = self.params.velocity_skew.value();
        self.lower_threshold = self.params.lower_threshold.value();
        self.upper_threshold = self.params.upper_threshold.value();
    }

    fn update_velocity_parameters_smoothed_with_step(&mut self, step: u32) {
        self.vel_min = self.params.velocity_min.smoothed.next_step(step);
        self.vel_max = self.params.velocity_max.smoothed.next_step(step);
        self.vel_skew = self.params.velocity_skew.smoothed.next_step(step);
        self.lower_threshold = self.params.lower_threshold.smoothed.next_step(step);
        self.upper_threshold = self.params.upper_threshold.smoothed.next_step(step);
    }

    // Get the normalized time within a measure (between 0.0 and 1.0) depending on the current
    // progress_in_samples or the bar_pos.
    fn get_normalized_position_in_bar(&self) -> f32 {
        // Calculate the current time in seconds from the current progress_in_samples
        let time = self.progress_in_samples as f32 / self.sample_rate;
        // Get the normalized time within a measure (between 0.0 and 1.0)
        if self.params.use_position.value() {
            self.bar_pos
        } else {
            let pos = time.rem_euclid(self.metric_duration) / self.metric_duration;
            self.params.displayed_position.store(pos, SeqCst);
            pos
        }
    }

    // set metric_duration to length of a quarter note times the slider
    fn set_metric_duration_for_bpm(&mut self, tempo: Option<f64>) {
        let one_crotchet = 60.0 / tempo.unwrap_or(60.0);
        self.metric_duration *= one_crotchet as f32;
    }

    // TODO this shouldn't be a method
    fn is_accent(&self, indisp_value: usize) -> bool {
        let skew = self.vel_skew;
        let nr_beats = self.params.current_nr_of_beats.load(SeqCst) as f32;
        let nr_of_accents = (skew * nr_beats).round() as usize;
        indisp_value >= nr_of_accents
    }

    fn calculate_current_velocity(&self, indisp_value: usize) -> f32 {
        // The current velocity Parameters
        let v_min: f32 = self.vel_min.min(self.vel_max) / 127.0;
        let v_max: f32 = self.vel_max / 127.0;
        let skew = self.vel_skew;
        let many_velocities = self.params.many_velocities.load(SeqCst);
        // Velocity in range 0.0 - 1.0,
        let normalized_vel =
            if many_velocities {
                (1.0 / (indisp_value + 1) as f32).powf(2.0*(1.0 - skew))
            } else if self.is_accent(indisp_value) {
                v_min
            } else {
                v_max
            };
        // rescaled by vel_min and vel_max parameters
        if v_min == v_max {
            v_min
        } else {
            rescale(normalized_vel, 0.0, 1.0, v_min, v_max, true)
                .unwrap_or(0.8)
        }
    }

    fn get_beat_idx_from_durations(&self, mut durations: impl Iterator<Item=f32>) -> (usize, f32, usize) {
        let position = self.get_normalized_position_in_bar();
        let mut current_beat_idx: usize = 0;
        let mut current_beat_duration_sum: f32 = 0.0;
        let mut nr_beats = 0;

        while let Some(dur) = durations.next() {
            nr_beats += 1;

            if current_beat_duration_sum + dur >= position {
                nr_beats += durations.count();
                break;
            }

            current_beat_duration_sum += dur;
            current_beat_idx += 1;
        }

        (current_beat_idx, current_beat_duration_sum, nr_beats)
    }

    /// return a tuple with the index of the current beat, the normalized duration up until that beat,
    /// the indispensability value for that beat and whether the thresholds would currently let
    /// a note through.
    fn get_current_indisp_data(&self) -> (usize, f32, usize, bool, BeatOrigin) {
        let metric_data = &self.params.combined_metre_data.lock().unwrap();
        let metric_data_a = metric_data.metre_a();
        let metric_data_b = metric_data.metre_b();
        let interpolation_data = metric_data.interpolation_data();
        let interpolate_indisp = &self.params.interpolate_indisp.load(SeqCst);
        let max_len = metric_data_a.durations.len().max(metric_data_b.durations.len());
        let same_length: bool = metric_data_a.durations.len() == metric_data_b.durations.len();

        let current_beat_idx_a;
        let current_beat_idx_b;
        let current_beat_idx;
        let current_beat_duration_sum;
        let current_beat_origin: BeatOrigin;

        // TODO no_many_velocities + don't_interpolate is a bit confusing for the user

        if self.interpolate_durs {
            let durations = interpolation_data.get_interpolated_durations(self.interpolate);
            let (idx, sum, total_nr_beats) = self.get_beat_idx_from_durations(durations);

            current_beat_idx_a = idx;
            current_beat_idx_b = idx;
            current_beat_idx = idx;
            current_beat_duration_sum = sum;
            current_beat_origin = Both;
            self.params.current_nr_of_beats.store(total_nr_beats, SeqCst);
        } else {
            let durations = metric_data.get_interleaved_durations(self.interpolate);
            let (idx, sum, total_nr_beats) = self.get_beat_idx_from_durations(durations);
            (current_beat_idx_a, _, _) = self.get_beat_idx_from_durations(metric_data_a.durations.iter().copied());
            (current_beat_idx_b, _, _) = self.get_beat_idx_from_durations(metric_data_b.durations.iter().copied());

            current_beat_idx = idx;
            current_beat_duration_sum = sum;
            current_beat_origin = match self.interpolate {
                x if x <= 0.0 => MetreA,
                x if x >= 1.0 => MetreB,
                _ => interpolation_data.unique_start_time_origins()[idx],
            };
            self.params.current_nr_of_beats.store(total_nr_beats, SeqCst);
        }

        let indisp_val_temp: f32 =
            if *interpolate_indisp || current_beat_origin == Both {
                dry_wet(
                    *metric_data_a.value.get(current_beat_idx_a).unwrap_or(&0),
                    *metric_data_b.value.get(current_beat_idx_b).unwrap_or(&0),
                    self.interpolate)
            } else {
                match current_beat_origin {
                    MetreA => *metric_data_a.value.get(current_beat_idx_a).unwrap_or(&0) as f32,
                    MetreB => *metric_data_b.value.get(current_beat_idx_b).unwrap_or(&0) as f32,
                    // this should never occur:
                    _ => 0.0,
                }
            };

        // TODO is this a good method for round/ceil? needs more testing!
        let indisp_val: usize = if same_length {
            indisp_val_temp.round() as usize
        } else {
            indisp_val_temp.ceil() as usize
        };

        (current_beat_idx,
         current_beat_duration_sum,
         indisp_val,
         self.is_indisp_val_within_thresholds(indisp_val, max_len - 1),
         current_beat_origin)
    }

    /// Get a MIDI event and either return none (filter it) or return it with a new velocity
    /// value (according to the current metric position).
    fn process_event<S: SysExMessage>(&mut self, event: NoteEvent<S>) -> Option<NoteEvent<S>> {
        let (_,_, indisp_val, let_through, _) = self.get_current_indisp_data();
        let vel: f32 = self.calculate_current_velocity(indisp_val);

        // Return new MIDI event (or None)
        match event {
            NoteEvent::NoteOn {
                timing,
                voice_id,
                channel,
                note,
                ..
            } => { if let_through {
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
        const NR_SAMPLES_FOR_START_OF_BEAT: u64 = 100;
        let mut current_sample: u32 = 0;
        let buffer_len = buffer.samples();
        let process_events: bool = !self.params.send_midi.value();

        // Handle the reset_phase button:
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

        // either process events or send some
        // TODO is it possible to process events while sending them also??
        if process_events {
            let mut last_note_was_let_through = true;
            let mut elapsed_samples: u32;

            // reset progress when playback stops. // TODO maybe do this for every sample, not once per bufffer?
            // TODO reset counter when self.was_playing=true and is_playing=false, only progress when is_playing...
            self.maybe_reset_progress(context.transport().playing);

            // handle all incoming events
            while let Some(event) = context.next_event() {
                // samples since last event
                elapsed_samples = event.timing() - current_sample;
                // update progress // TODO only when playing?
                self.progress_in_samples += elapsed_samples as u64;
                current_sample += elapsed_samples;

                // get all parameters for this event
                if elapsed_samples > 0 {
                    self.update_velocity_parameters_smoothed_with_step(elapsed_samples);
                    self.metric_duration = self.params.metric_dur_selector.smoothed.next_step(elapsed_samples);
                    self.bar_pos = self.params.bar_position.smoothed.next_step(elapsed_samples);
                    self.interpolate = self.params.interpolate_a_b.smoothed.next_step(elapsed_samples);
                } else {
                    self.update_velocity_parameters();
                    self.metric_duration = self.params.metric_dur_selector.value();
                    self.bar_pos = self.params.bar_position.value();
                    self.interpolate = self.params.interpolate_a_b.value();
                }

                if self.params.use_bpm.value() {
                    self.set_metric_duration_for_bpm(context.transport().tempo);
                }

                match event {
                    NoteEvent::NoteOn { .. } => {
                        if let Some(event) = self.process_event(event) {
                            context.send_event(event);
                            last_note_was_let_through = true;
                        } else {
                            last_note_was_let_through = false;
                        }
                    },
                    NoteEvent::NoteOff { .. } => {
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
            self.update_velocity_parameters_smoothed_with_step(elapsed_samples);
            self.metric_duration = self.params.metric_dur_selector.smoothed.next_step(elapsed_samples);
            self.bar_pos = self.params.bar_position.smoothed.next_step(elapsed_samples);
            self.interpolate = self.params.interpolate_a_b.smoothed.next_step(elapsed_samples);
        } else {
            let output_one_pitch = self.params.midi_out_one_note.load(SeqCst);
            let many_velocities = self.params.many_velocities.load(SeqCst);
            self.interpolate_durs = self.params.interpolate_durations.load(SeqCst);
            self.interpolate_indisp = self.params.interpolate_indisp.load(SeqCst);
            // Since the metric duration might change while doing this, maybe it's easiest to just
            // loop through all samples and individually check, whether we want to send a note.
            for sample in 0..buffer_len {
                // reset progress when not playing
                self.maybe_reset_progress(context.transport().playing);

                self.metric_duration = self.params.metric_dur_selector.smoothed.next_step(1);
                self.bar_pos = self.params.bar_position.smoothed.next_step(1);
                self.interpolate = self.params.interpolate_a_b.smoothed.next_step(1);
                // TODO this can be done somewhere else and less often
                self.update_velocity_parameters_smoothed_with_step(1);

                if self.params.use_bpm.value() {
                    self.set_metric_duration_for_bpm(context.transport().tempo);
                }

                let (current_beat_idx, current_beat_duration_sum, indisp_val, let_through, origin) =
                    self.get_current_indisp_data();

                let beat_first_sample: u64 =
                    (current_beat_duration_sum
                        * self.metric_duration
                        * self.sample_rate)
                        .floor() as u64;

                let nth_sample_in_bar: u64 = (self.get_normalized_position_in_bar()
                    * self.metric_duration
                    * self.sample_rate)
                    .floor() as u64;

                let nth_sample_of_beat: u64 = nth_sample_in_bar.saturating_sub(beat_first_sample);

                // Are we at the beginning of a beat?
                if nth_sample_of_beat < NR_SAMPLES_FOR_START_OF_BEAT {
                    // Send midi when we haven't already sent a note for this idx
                    if self.last_sent_beat_idx != current_beat_idx as i32 && let_through {
                        let vel = {
                            let tmp_vel = self.calculate_current_velocity(indisp_val);

                            if self.interpolate_indisp {
                                tmp_vel
                            } else {
                                match origin {
                                    Both => tmp_vel,
                                    MetreA => dry_wet(tmp_vel, 0.0, self.interpolate),
                                    MetreB => dry_wet(0.0, tmp_vel, self.interpolate),
                                }
                            }
                        };
                        let note = 60
                            + if output_one_pitch {
                            0
                        } else if many_velocities {
                            indisp_val as u8
                        } else if self.is_accent(indisp_val) {
                            0
                        } else {
                            1
                        };

                        context.send_event(
                            NoteEvent::NoteOn {
                                timing: sample as u32,
                                velocity: vel,
                                channel: 0,
                                note,
                                voice_id: None
                            });

                        self.last_sent_beat_idx = current_beat_idx as i32;

                        // send a Note Off into self.note_off_buffer
                        if let Some((n, delay)) = self.note_off_buffer.iter_mut().find(|&&mut (_, y)| y<0) {
                            *delay = sample as i64 + (0.1 * self.sample_rate).floor() as i64;
                            *n = note;
                        }
                    }
                } else {
                    self.last_sent_beat_idx = -1
                }
                if context.transport().playing {
                    self.progress_in_samples += 1;
                }
            }
            // Handle Note Offs
            for (note, delay) in self.note_off_buffer.iter_mut() {
                if *delay >= buffer_len as i64 {
                    *delay -= buffer_len as i64
                } else if *delay >= 0 {
                    context.send_event(
                        NoteEvent::NoteOff {
                            timing: *delay as u32,
                            voice_id: None,
                            channel: 0,
                            note: *note,
                            velocity: 0.0,
                        });
                    *delay = -1
                }
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for MetreFiddler {
    const CLAP_ID: &'static str = "leonfocker.metrefiddler";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Midi processing based on metric structures");
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
