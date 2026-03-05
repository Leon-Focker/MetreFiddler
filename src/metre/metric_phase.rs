/// I want progress_in_samples and metric_duration_samples to basically function like a
/// rational number (when taking progress/duration), so they have to be kept in sync:
/// -> When metric duration is changed, the progress is updated to keep the current ratio.
/// Thus, I'm making this its own struct, so I can keep the fields private...
pub struct MetricPhase {
    progress_in_samples: u64,
    metric_duration_samples: u64,
    metric_phase: f32,
}

impl Default for MetricPhase {
    fn default() -> Self {
        Self {
            progress_in_samples: 0,
            metric_duration_samples: 1,
            metric_phase: 0.0,
        }
    }
}

impl MetricPhase {
    pub fn reset(&mut self) {
        self.progress_in_samples = 0;
    }

    pub fn progress_in_samples(&self) -> u64 {
        self.progress_in_samples
    }

    pub fn metric_duration_samples(&self) -> u64 {
        self.metric_duration_samples
    }

    pub fn metric_phase(&self) -> f32 {
        self.metric_phase
    }

    pub fn increment(&mut self) {
        self.progress_in_samples += 1;
        if self.progress_in_samples >= self.metric_duration_samples {
            self.progress_in_samples -= self.metric_duration_samples;
        }
        self.update_phase()
    }

    fn update_phase(&mut self) {
        self.metric_phase = (self.progress_in_samples % self.metric_duration_samples) as f32 / self.metric_duration_samples as f32;
    }

    pub fn set_metric_duration(&mut self, new_metric_duration: f32, sample_rate: f32, use_bpm: bool, tempo: Option<f64>, retain_phase: bool) {
        let bpm_multiplier = if use_bpm {
            let one_crotchet = 60.0 / tempo.unwrap_or(60.0);
            one_crotchet as f32
        } else {
            1.0
        };

        let new_metric_duration_samples = (new_metric_duration * sample_rate * bpm_multiplier).round() as u64;

        if new_metric_duration_samples != self.metric_duration_samples {
            self.metric_duration_samples = new_metric_duration_samples;
            if retain_phase {
                // Update progress_in_samples to retain phase:
                self.progress_in_samples = (self.metric_phase * new_metric_duration_samples as f32).round() as u64;                
            } else {
                self.update_phase();
            }
        }
    }
}