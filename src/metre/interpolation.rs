use std::cmp::max;
use nih_plug::{nih_dbg, nih_log};
use serde::{Deserialize, Serialize};
use vizia_plug::vizia::prelude::Data;
use crate::metre::rqq::RQQ;
use crate::metre_data::MetreData;
use crate::util::{approx_eq, get_start_times};

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
        value: generate_interpolation_data_aux(durations_a, durations_b, gnsm_a, gnsm_b)
            .iter().map(|&element| if let Some((i_a, i_b)) = element {
            (*durations_a.get(i_a).unwrap_or(&0.0), *durations_b.get(i_b).unwrap_or(&0.0))
        } else {
            (0.0, 0.0)
        })
            .collect()
    }
    // TODO? sort by index...
}

fn pair_identical_start_times(result: &mut [Option<(usize, usize)>], durations_a: &[f32], durations_b: &[f32]) {
    let starts_a = get_start_times(durations_a);
    let starts_b = get_start_times(durations_b);

    for (i, &x) in starts_a.iter().enumerate() {
        if let Some(pos) = starts_b.iter().position(|&y| approx_eq(x, y, 0.001)) {
            result[i] = Some((i, pos))
        }
    }
}

// TODO I have no clue whether this function works...?
fn pair_higher_strata_by_time(durations_a: &[f32], durations_b: &[f32], gnsm_a: &[usize]) -> (usize, usize) {
    let starts_a = get_start_times(durations_a);
    let starts_b = get_start_times(durations_b);

    // find the indices which belong to the highest stratum
    let highest_stratum = *gnsm_a.iter().max().unwrap_or(&1);
    let idx_a = gnsm_a.iter().rposition(|&x| x == highest_stratum).expect("something went wrong in fn pair_higher_strata_by_time");
    let start_time_a = starts_a[idx_a];
    let idx_b = starts_b.iter()
        .map(| &start| (start - start_time_a).abs())
        .enumerate()
        .min_by(| (_, x), (_, y) |x.cmp(y))
        .unwrap_or((0, 0.0))
        .0;

    (idx_a, idx_b)
}


fn generate_interpolation_data_aux(durations_a: &[f32], durations_b: &[f32], gnsm_a: &[usize], gnsm_b: &[usize]) -> Vec<Option<(usize, usize)>> {
    let len_a = durations_a.len();
    let len_b = durations_b.len();
    let max_len = len_a.max(len_b);
    let mut result = vec![None; max_len];

    // if same length or gnsm all 0
    // -> append 0.0 in the end if necessary
    // else:
    // if all values are None, check for same starttimes (this is only necessary once)
    // -> this fills some gaps
    // check whether some places are not set yet
    // all set -> return vec
    // not all set -> check for difference in length between unset passage
    // is <= 1 -> append 0.0 in the end
    // greater than 1 ->
    // if both have different strata left, match beats from the same strata (with gnsm)
    // if both have no strata left, append 0.0 in the end
    // else find match for the beat from higher strata by closest start-time...

    //  if same length or gnsm all 0 -> append 0.0 in the end if necessary (no further subdivision).
    if len_a == len_b || (gnsm_a.iter().all(|&x| x == 0) &&  gnsm_b.iter().all(|&x| x == 0)) {
        result = (0..max_len).map(|i| Some((i, i))).collect();
        // if all values are None, check for same start-times (this is only necessary once)
    } else if result.iter().all(|x|x.is_none()) {
        pair_identical_start_times(&mut result, durations_a, durations_b);
        // check this only after similar start-times have been checked for...
    } else if len_a.abs_diff(len_b) == 1 {
        result = (0..max_len).map(|i| Some((i, i))).collect();
        //  Difference in length in > 1
    } else {
        let no_strata_left_a = gnsm_a.iter().all(|&x| x == 0);
        let no_strata_left_b = gnsm_b.iter().all(|&x| x == 0);

        // if both have different strata left, match beats from the same strata
        if !no_strata_left_a && !no_strata_left_b {
            // TODO
            //
            // else find match for the beat from higher strata by closest start-time...
            // TODO does this work?
        } else if !no_strata_left_a && no_strata_left_b {
            result[result.iter().position(|&x| x.is_none())] = pair_higher_strata_by_time(durations_a, durations_b, gnsm_a);
        } else if no_strata_left_a && !no_strata_left_b {
            result[result.iter().position(|&x| x.is_none())] = pair_higher_strata_by_time(durations_b, durations_a, gnsm_b);
        } else {
            result = (0..max_len).map(|i| Some((i, i))).collect();
        }
    }

    // check whether some places are not set yet
    if !result.iter().all(|x| x.is_some()) {

        // TODO get unset passages
        // for each passage: recursive call and wiedereingliederung.
        result = (0..max_len).map(|i| Some((i, i))).collect();
        result
    } else { result }
}