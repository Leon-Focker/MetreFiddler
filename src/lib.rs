use nih_plug::prelude::*;
use std::sync::{Arc};
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use crate::metre::beat_origin::BeatOrigin;
use crate::metre::beat_origin::BeatOrigin::*;
use crate::params::{MetreFiddlerParams, ParamsSnapShot};
use crate::util::{dry_wet, rescale};

mod editor;
mod metre;
mod gui;
mod util;
mod params;

struct MetreFiddler {
    params: Arc<MetreFiddlerParams>,
    params_snapshot: ParamsSnapShot,

    sample_rate: f32,
    progress_in_samples: u64, // TODO maybe this should be reset after each full measure??
    last_reset_phase_value: bool,
    last_sent_beat_idx: i32,
    note_off_buffer: Vec<Option<(u8, i32, i64)>>,
    was_playing: bool,
}

// TODO check concurrency of current_nr_beats

impl Default for MetreFiddler {
    fn default() -> Self {
        let default_params = Arc::new(MetreFiddlerParams::default());
        Self {
            params: default_params.clone(),
            params_snapshot: ParamsSnapShot::default(),
            sample_rate: 1.0,
            last_reset_phase_value: false,
            last_sent_beat_idx: -1,
            note_off_buffer: vec![None; 8],
            was_playing: false,
            progress_in_samples: 0,
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
        indisp_val >= (self.params_snapshot.lower_threshold.min(self.params_snapshot.upper_threshold) * max_indisp_val as f32) as usize
            && indisp_val <= (self.params_snapshot.upper_threshold * max_indisp_val as f32) as usize
    }

    // Get the normalized time within a measure (between 0.0 and 1.0) depending on the current
    // progress_in_samples or the bar_pos.
    fn get_normalized_position_in_bar(&self) -> f32 {
        // Calculate the current time in seconds from the current progress_in_samples
        let time = self.progress_in_samples as f32 / self.sample_rate;
        // Get the normalized time within a measure (between 0.0 and 1.0)
        if self.params.use_position.value() {
            self.params_snapshot.bar_pos
        } else {
            let duration = self.params_snapshot.metric_duration;
            let pos = time.rem_euclid(duration) / duration;
            self.params.displayed_position.store(pos, Relaxed);
            pos
        }
    }

    // set metric_duration to length of a quarter note times the slider
    fn set_metric_duration_for_bpm(&mut self, tempo: Option<f64>) {
        let one_crotchet = 60.0 / tempo.unwrap_or(60.0);
        self.params_snapshot.metric_duration *= one_crotchet as f32;
    }

    // TODO this shouldn't be a method
    fn is_accent(&self, indisp_value: usize) -> bool {
        let skew = self.params_snapshot.vel_skew;
        let nr_beats = self.params.current_nr_of_beats.load(Relaxed) as f32;
        let nr_of_accents = (skew * nr_beats).round() as usize;
        indisp_value >= nr_of_accents
    }

    fn calculate_current_velocity(&self, indisp_value: usize) -> f32 {
        // The current velocity Parameters
        let v_min: f32 = self.params_snapshot.vel_min.min(self.params_snapshot.vel_max) / 127.0;
        let v_max: f32 = self.params_snapshot.vel_min.max(self.params_snapshot.vel_max) / 127.0;
        let skew = self.params_snapshot.vel_skew;
        let many_velocities = self.params_snapshot.many_velocities;
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
    /// the indispensability value for that beat, whether the thresholds would currently let
    /// a note through and the Origin of the current Beat.
    fn get_current_indisp_data(&self) -> (usize, f32, usize, bool, BeatOrigin) {
        // TODO ideally we never want to lock in the audio thread, can this be replaced with rtrb?
        let metric_data = &self.params.combined_metre_data.lock().unwrap();
        let metric_data_a = metric_data.metre_a();
        let metric_data_b = metric_data.metre_b();
        let interpolation_data = metric_data.interpolation_data();
        let max_len = metric_data_a.durations.len().max(metric_data_b.durations.len());
        let same_length: bool = metric_data_a.durations.len() == metric_data_b.durations.len();

        let current_beat_idx_a;
        let current_beat_idx_b;
        let current_beat_idx;
        let current_beat_duration_sum;
        let current_beat_origin: BeatOrigin;

        // TODO no_many_velocities + don't_interpolate is a bit confusing for the user

        if self.params_snapshot.interpolate_durs {
            let durations = interpolation_data.get_interpolated_durations(self.params_snapshot.interpolate);
            let (idx, sum, total_nr_beats) = self.get_beat_idx_from_durations(durations);

            current_beat_idx_a = idx;
            current_beat_idx_b = idx;
            current_beat_idx = idx;
            current_beat_duration_sum = sum;
            current_beat_origin = Both;
            self.params.current_nr_of_beats.store(total_nr_beats, SeqCst);
        } else {
            let durations = metric_data.get_interleaved_durations(self.params_snapshot.interpolate);
            let (idx, sum, total_nr_beats) = self.get_beat_idx_from_durations(durations);
            (current_beat_idx_a, _, _) = self.get_beat_idx_from_durations(metric_data_a.durations.iter().copied());
            (current_beat_idx_b, _, _) = self.get_beat_idx_from_durations(metric_data_b.durations.iter().copied());

            current_beat_idx = idx;
            current_beat_duration_sum = sum;
            current_beat_origin = match self.params_snapshot.interpolate {
                x if x <= 0.0 => MetreA,
                x if x >= 1.0 => MetreB,
                _ => interpolation_data.unique_start_time_origins()[idx],
            };
            self.params.current_nr_of_beats.store(total_nr_beats, SeqCst);
        }

        let indisp_val_temp: f32 =
            if self.params_snapshot.interpolate_indisp|| current_beat_origin == Both {
                dry_wet(
                    *metric_data_a.value.get(current_beat_idx_a).unwrap_or(&0),
                    *metric_data_b.value.get(current_beat_idx_b).unwrap_or(&0),
                    self.params_snapshot.interpolate)
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
    fn process_note_event<S: SysExMessage>(&mut self, event: NoteEvent<S>) -> Option<NoteEvent<S>> {
        match event {
            NoteEvent::NoteOn {
                timing,
                voice_id,
                channel,
                note,
                ..
            } => {
                let (_,_, indisp_val, let_through, _) = self.get_current_indisp_data();
                let vel: f32 = self.calculate_current_velocity(indisp_val);

                if let_through {
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
        let nr_samples_for_start_of_beat: u64 = (self.sample_rate / 500.0).ceil() as u64;
        let mut next_event = context.next_event();
        let buffer_len = buffer.samples();

        // Get all plain parameter values once here
        self.params_snapshot = self.params.snapshot();

        // reset progress when playback stops. // TODO rethink while reworking how progress works
        self.maybe_reset_progress(context.transport().playing);

        // TODO this is still dodgy and only happens once per buffer
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

        for (sample_id, _) in buffer.iter_samples().enumerate() {
            // update Parameters with smoothing
            self.params_snapshot.metric_duration = self.params.metric_dur_selector.smoothed.next();
            self.params_snapshot.bar_pos = self.params.bar_position.smoothed.next();
            self.params_snapshot.interpolate = self.params.interpolate_a_b.smoothed.next();

            // Convert metric_duration when using bpm
            if self.params_snapshot.use_bpm {
                self.set_metric_duration_for_bpm(context.transport().tempo);
            }

            // loop through events at this time
            while let Some(event) = next_event {
                if event.timing() > sample_id as u32 {
                    break;
                }

                match event {
                    NoteEvent::NoteOn { .. } => {
                        if let Some(event) = self.process_note_event(event) {
                            context.send_event(event);
                        }
                    },
                    // it's safest to just let all NoteOffs through, right?
                    NoteEvent::NoteOff {..} => {
                        context.send_event(event)
                    },
                    _ => context.send_event(event),
                }

                next_event = context.next_event();
            }

            // Send Midi
            if self.params.send_midi.value() {
                let (current_beat_idx, current_beat_duration_sum, indisp_val, let_through, origin) =
                    self.get_current_indisp_data();

                let beat_first_sample: u64 =
                    (current_beat_duration_sum
                        * self.params_snapshot.metric_duration
                        * self.sample_rate)
                        .floor() as u64;

                let nth_sample_in_bar: u64 = (self.get_normalized_position_in_bar()
                    * self.params_snapshot.metric_duration
                    * self.sample_rate)
                    .floor() as u64;

                let nth_sample_of_beat: u64 = nth_sample_in_bar.saturating_sub(beat_first_sample);

                // Are we at the beginning of a beat?
                if nth_sample_of_beat < nr_samples_for_start_of_beat {
                    // Send midi when we haven't already sent a note for this idx
                    if self.last_sent_beat_idx != current_beat_idx as i32 && let_through {
                        let vel = {
                            let tmp_vel = self.calculate_current_velocity(indisp_val);

                            if self.params_snapshot.interpolate_indisp {
                                tmp_vel
                            } else {
                                match origin {
                                    Both => tmp_vel,
                                    MetreA => dry_wet(tmp_vel, 0.0, self.params_snapshot.interpolate),
                                    MetreB => dry_wet(0.0, tmp_vel, self.params_snapshot.interpolate),
                                }
                            }
                        };
                        let note = 60
                            + if self.params_snapshot.output_one_pitch {
                            0
                        } else if self.params_snapshot.many_velocities {
                            indisp_val as u8
                        } else if self.is_accent(indisp_val) {
                            0
                        } else {
                            1
                        };

                        context.send_event(
                            NoteEvent::NoteOn {
                                timing: sample_id as u32,
                                velocity: vel,
                                channel: 0,
                                note,
                                voice_id: Some(sample_id as i32),
                            });

                        self.last_sent_beat_idx = current_beat_idx as i32;

                        // send a Note Off into self.note_off_buffer
                        let release_timing = sample_id as i64 + (0.1 * self.sample_rate).floor() as i64;
                        if let Some(slot) = self.note_off_buffer.iter_mut().find(|e| e.is_none()) {
                            *slot = Some((note, sample_id as i32, release_timing));
                        }
                    }
                } else {
                    self.last_sent_beat_idx = -1
                }
            }

            // update progress
            if context.transport().playing {
                self.progress_in_samples += 1;
            }
        }

        // Handle Note Offs
        for event in self.note_off_buffer.iter_mut() {
            if let Some((note, id, release_timing)) = event {
                if *release_timing >= buffer_len as i64 {
                    *release_timing -= buffer_len as i64;
                } else {
                    context.send_event(
                        NoteEvent::NoteOff {
                            timing: *release_timing as u32,
                            voice_id: Some(*id),
                            channel: 0,
                            note: *note,
                            velocity: 0.0,
                        });

                    *event = None;
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
