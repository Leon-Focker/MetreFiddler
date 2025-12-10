use nih_plug::prelude::*;
use std::sync::{Arc};
use std::sync::atomic::Ordering::SeqCst;
use crate::params::MetreFiddlerParams;
use crate::util::{decider, dry_wet, rescale};
use nih_log;

mod editor;
mod metre_data;
mod metre;
mod gui;
mod util;
mod params;

struct MetreFiddler {
    params: Arc<MetreFiddlerParams>,
    sample_rate: f32,
    progress_in_samples: u64,
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
        let one_crotchet = 60.0 / if let Some(tempo) = tempo {
            tempo
        } else { 60.0 };
        self.metric_duration = one_crotchet as f32 * self.metric_duration;
    }

    fn calculate_current_velocity(&self, indisp_value: usize) -> f32 {
        // The current velocity Parameters
        let v_min: f32 = self.vel_min.min(self.vel_max) / 127.0;
        let v_max: f32 = self.vel_max / 127.0;
        // Velocity in range 0.0 - 1.0,
        let normalized_vel = (1.0 / (indisp_value + 1) as f32).powf(2.0*(1.0 - self.vel_skew));
        // rescaled by vel_min and vel_max parameters
        if v_min == v_max {
            v_min
        } else {
            rescale(normalized_vel, 0.0, 1.0, v_min, v_max, true)
                .unwrap_or(0.8)
        }
    }

    /// Get a MIDI event and either return none (filter it) or return it with a new velocity
    /// value (according to the current metric position).
    fn process_event<S: SysExMessage>(&mut self, event: NoteEvent<S>) -> Option<NoteEvent<S>> {
        let time_in_bar_normalized = self.get_normalized_position_in_bar();

        let indisp_val: usize;
        let max: usize;

        let interpol = self.interpolate;
        if interpol == 0.0 || interpol == 1.0 {
            let metric_data = if interpol == 0.0 {
                &self.params.metre_data_a.lock().unwrap()
            } else {
                &self.params.metre_data_b.lock().unwrap()
            };
            let metric_durations = &metric_data.durations;
            let indisp_ls = &metric_data.value;
            max = indisp_ls.len() - 1;

            // Get the index of the current indispensability value
            let indisp_idx: usize = if let Ok(idx) = decider(time_in_bar_normalized, &metric_durations) {
                idx as usize
            } else { 0 };
            // Get the actual indispensability value from the vector
            indisp_val = indisp_ls[indisp_idx];
        } else {
            let metric_data_a = &self.params.metre_data_a.lock().unwrap();
            let metric_data_b = &self.params.metre_data_b.lock().unwrap();
            let metric_durations_a = &metric_data_a.durations;
            let metric_durations_b = &metric_data_b.durations;
            let indisp_ls_a = &metric_data_a.value;
            let indisp_ls_b = &metric_data_b.value;
            max = (indisp_ls_a.len() - 1).max(indisp_ls_b.len() - 1);

            // Get the index of the current indispensability value
            let indisp_idx_a: usize = if let Ok(idx) = decider(time_in_bar_normalized, &metric_durations_a) {
                idx as usize
            } else { 0 };
            let indisp_idx_b: usize = if let Ok(idx) = decider(time_in_bar_normalized, &metric_durations_b) {
                idx as usize
            } else { 0 };
            // Get the actual indispensability value from the vector
            indisp_val = ((indisp_ls_a[indisp_idx_a] as f32 * (1.0 - interpol))
                +(indisp_ls_b[indisp_idx_b] as f32 * interpol))
                .ceil() as usize; // TODO decide if round or ceil (ceil prevents duplicates better...?)
        }

        let vel: f32 = self.calculate_current_velocity(indisp_val);

        // Return new MIDI event (or None)
        match event {
            NoteEvent::NoteOn {
                timing,
                voice_id,
                channel,
                note,
                ..
            } => { if self.is_indisp_val_within_thresholds(indisp_val, max) {
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
        // TODO is it possible to process evens while sending them also??
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

                // TODO would love to retrieve this just once every buffer but we're also borrowing self mutably...
                let metric_data_a = &self.params.metre_data_a.lock().unwrap();
                let metric_data_b = &self.params.metre_data_b.lock().unwrap();
                let max_len = metric_data_a.durations.len().max(metric_data_b.durations.len());

                let mut current_beat_idx: usize = 0;
                let mut current_duration_sum = 0.0;
                loop {
                    let dur_a = *metric_data_a.durations.get(current_beat_idx).unwrap_or(&0.0);
                    let dur_b = *metric_data_b.durations.get(current_beat_idx).unwrap_or(&0.0);
                    let dur = dry_wet(dur_a, dur_b, self.interpolate);
                    current_duration_sum += dur;
                    if current_beat_idx >= max_len || current_duration_sum >= self.get_normalized_position_in_bar() {
                        current_duration_sum -= dur;
                        break
                    } else {
                        current_beat_idx += 1;
                    }
                };

                let beat_first_sample: u64 =
                    (current_duration_sum
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
                    if self.last_sent_beat_idx != current_beat_idx as i32 {
                        // Get the actual indispensability value from the vector
                        let indisp_val: usize = dry_wet(
                            *metric_data_a.value.get(current_beat_idx).unwrap_or(&0),
                            *metric_data_b.value.get(current_beat_idx).unwrap_or(&0),
                            self.interpolate)
                            .ceil() as usize; // TODO decide if round or ceil (ceil prevents duplicates better...?)
                        let max = (metric_data_a.value.len() - 1).max(metric_data_b.value.len() - 1);

                        // check thresholds...
                        if self.is_indisp_val_within_thresholds(indisp_val, max) {

                            let vel =  self.calculate_current_velocity(indisp_val);
                            let note = 60 + indisp_val as u8;

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
