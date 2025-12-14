use std::cmp::max;
use nih_plug::{nih_dbg, nih_log};
use serde::{Deserialize, Serialize};
use vizia_plug::vizia::prelude::Data;
use crate::metre::rqq::RQQ;
use crate::metre_data::MetreData;
use crate::util::get_start_times;

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

pub fn generate_interpolation_data(durations_a: &[f32], durations_b: &[f32], gnsm_a: &[usize], gnsm_b: &[usize]) -> InterpolationData {
    InterpolationData {
        value: generate_interpolation_data_aux(durations_a, durations_b, gnsm_a, gnsm_b, true)
            .iter().map(|&element| if let Some(val) = element { val } else { (0.0, 0.0) }).collect()
    }
}

fn generate_interpolation_data_aux(durations_a: &[f32], durations_b: &[f32], gnsm_a: &[usize], gnsm_b: &[usize], check_for_start_times: bool) -> Vec<Option<(f32, f32)>> {
    let len_a = durations_a.len();
    let len_b = durations_b.len();
    let max_len = len_a.max(len_b);
    let mut result = vec![None; max_len];

    // if same length or gnsm all 0
    // -> append 0.0 in the end if necessary
    // else:
    // if check_for_start_times. check for same starttimes (this is only necessary once, thus the bool)
    // -> this fills some gaps
    // check whether some places are not set yet
    // all set -> return vec
    // not all set -> check for difference in length between unset passage
    // is <= 1 -> append 0.0 in the end
    // greater than 1 ->
    // if both have different strata left, match beats from the same strata (with gnsm)
    // if both have no strata left, append 0.0 in the end
    // else find match for the beat from higher strata by closest start-time...

    // no further subdivision needed when both structures are of the same length or only on the
    // lowest metric strata
    if len_a == len_b || (gnsm_a.iter().all(|&x| x == 0) &&  gnsm_b.iter().all(|&x| x == 0)) {
        result = (0..max_len).map(|i|
            Some((*durations_a.get(i).unwrap_or(&0.0),
                  *durations_b.get(i).unwrap_or(&0.0))))
            .collect();
    } else {
        // result = vec![(-1.0,-1.0); max_len];
        let starts_a = get_start_times(durations_a);
        let starts_b = get_start_times(durations_b);

        result = (0..max_len).map(|i|
            Some((*durations_a.get(i).unwrap_or(&0.0),
                  *durations_b.get(i).unwrap_or(&0.0))))
            .collect();
    }
    result
}