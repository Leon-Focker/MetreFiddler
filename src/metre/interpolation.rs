use std::cmp::max;
use nih_plug::nih_log;
use serde::{Deserialize, Serialize};
use vizia_plug::vizia::prelude::Data;
use crate::metre::rqq::RQQ;
use crate::metre_data::MetreData;

/// Holds pairs of durations (one for each of two MetreDatas). If one metric structure has more
/// beats than the other, some of its beats will be paired with 0.0.
#[derive(Debug, Serialize, Deserialize, Clone, Data)]
pub struct InterpolationData {
    pub value: Vec<(f32, f32)>,
}

impl Default for InterpolationData {
    fn default() -> Self {
        Self {
            value: vec![(0.25, 0.25); 4],
        }
    }
}

pub fn generate_interpolation_data(durations_a: &[f32], durations_b: &[f32]) -> InterpolationData {
    // TODO cooler (smarter) interpolation?
    let max_len = durations_a.len().max(durations_b.len());
    let result: Vec<(f32, f32)> = (0..max_len).map(|i|
        (*durations_a.get(i).unwrap_or(&0.0),
         *durations_b.get(i).unwrap_or(&0.0)))
        .collect();

    InterpolationData { value: result }
}