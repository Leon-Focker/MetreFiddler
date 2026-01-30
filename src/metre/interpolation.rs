use nih_plug::log::error;
use serde::{Deserialize, Serialize};
use vizia_plug::vizia::prelude::Data;
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
    assert_eq!(durations_a.len(), gnsm_a.len());
    assert_eq!(durations_b.len(), gnsm_b.len());
    InterpolationData {
        // Get pairs of indices and map them to the actual durations from A and B.
        value: generate_interpolation_data_aux(durations_a, durations_b, gnsm_a, gnsm_b, 0, 0)
            .iter().map(|&(idx_a, idx_b)|
            (
                if let Some(idx) = idx_a {
                    *durations_a.get(idx).unwrap_or(&0.0)
                } else { 0.0 },
                if let Some(idx) = idx_b {
                    *durations_b.get(idx).unwrap_or(&0.0)
                } else { 0.0 })
        )
            .collect()
    }
}

fn first_free_idx(vec: &[(Option<usize>, Option<usize>)]) -> usize {
    vec.iter().position(|&(x, y)| x.is_none() && y.is_none()).unwrap_or(vec.len() - 1)
}

fn all_free(vec: &[(Option<usize>, Option<usize>)]) -> bool {
    vec.iter().all(|&(x, y)| x.is_none() && y.is_none())
}

/// Collect ascending indices from range of each section, choose out-of-bounds index if necessary.
fn ascending_indices_with_padding(len: usize, len_a: usize, len_b: usize, first_a: usize, first_b: usize) -> Vec<(Option<usize>, Option<usize>)> {
    (0..len).map(|i| (
        if i < len_a {
            Some( i + first_a)
        } else { None },
        if i < len_b {
            Some( i + first_b)
        } else { None },
    )).collect()
}

/// Given durations A and B, look for identical start times. For each identical start time in both
/// sets of durations, get their indices and pair them into result.
fn pair_identical_start_times(result: &mut [(Option<usize>, Option<usize>)], durations_a: &[f32], durations_b: &[f32], idx_a_offset: usize, idx_b_offset: usize) {
    let starts_a = get_start_times(durations_a);
    let starts_b = get_start_times(durations_b);

    for (i, &x) in starts_a.iter().enumerate() {
        if let Some(pos) = starts_b.iter().position(|&y| approx_eq(x, y, 0.001)) {
            result[first_free_idx(result)]
                = (Some(i + idx_a_offset), Some(pos + idx_b_offset))
        }
    }
}

/// While durations A does have some metrical hierarchy indicated by gnsm_a, durations B does not.
/// Find the beat with the highest metrical value in durations A and pair it with the closest beat from B by start-time
fn pair_higher_stratum_by_time(durations_a: &[f32], durations_b: &[f32], gnsm_a: &[usize], idx_a_offset: usize, idx_b_offset: usize) -> (Option<usize>, Option<usize>) {
    let starts_a = get_start_times(durations_a);
    let starts_b = get_start_times(durations_b);

    // find the indices which belong to the highest stratum
    let highest_stratum = *gnsm_a.iter().max().unwrap_or(&1);
    let idx_a = gnsm_a.iter().rposition(|&x| x == highest_stratum).expect("something went wrong in fn pair_higher_stratum_by_time");
    let start_time_a = starts_a[idx_a];
    // get index for Start in B that's closest to start_time_a
    let idx_b = starts_b.iter()
        .map(| &start| (start - start_time_a).abs())
        .enumerate()
        .min_by(| (_, x), (_, y) |x.total_cmp(y))
        .unwrap_or((0, 0.0))
        .0;

    (Some(idx_a + idx_a_offset), Some(idx_b + idx_b_offset))
}

fn pair_highest_stratus (durations_a: &[f32], durations_b: &[f32], gnsm_a: &[usize], gnsm_b: &[usize], idx_a_offset: usize, idx_b_offset: usize) -> (Option<usize>, Option<usize>) {
    let starts_a = get_start_times(durations_a);
    let starts_b = get_start_times(durations_b);

    // find the indices which belong to the highest stratus
    let highest_stratum_a = *gnsm_a.iter().max().unwrap_or(&1);
    let idx_a = gnsm_a.iter().rposition(|&x| x == highest_stratum_a).expect("something went wrong in fn pair_highest_stratus ");
    let highest_stratum_b = *gnsm_b.iter().max().unwrap_or(&1);
    let idx_b = gnsm_b.iter().rposition(|&x| x == highest_stratum_b).expect("something went wrong in fn pair_highest_stratus ");

    (Some(idx_a + idx_a_offset), Some(idx_b + idx_b_offset))
}

/// Return a vector of pairs of indices.
fn generate_interpolation_data_aux(durations_a: &[f32], durations_b: &[f32], gnsm_a: &[usize], gnsm_b: &[usize], idx_a_offset: usize, idx_b_offset: usize) -> Vec<(Option<usize>, Option<usize>)> {
    let len_a = durations_a.len();
    let len_b = durations_b.len();
    let max_len = len_a.max(len_b);
    let no_strata_left_a = gnsm_a.iter().all(|&x| x == gnsm_a[0]);
    let no_strata_left_b = gnsm_b.iter().all(|&x| x == gnsm_b[0]);
    let mut result = vec![(None, None); max_len];

    // Apply one of the methods below (either complete result or match some indices),
    // then call recursively with empty subsections

    // If both sections are of the same length, or when all are of the same stratum:
    if len_a == len_b || (no_strata_left_a && no_strata_left_b) {
        result = ascending_indices_with_padding(max_len, len_a, len_b, idx_a_offset, idx_b_offset);
    } else {
        // try finding pairs via similar start-times
        if all_free(&result) {
            pair_identical_start_times(&mut result, durations_a, durations_b, idx_a_offset, idx_b_offset);
        }
        // if no matches were found and difference in length is just 1, append 0.0.
        // Else look for a more complicated method to match some pairs
        if all_free(&result) &&
            len_a.abs_diff(len_b) == 1 {
            result = ascending_indices_with_padding(max_len, len_a, len_b, idx_a_offset, idx_b_offset);
        } else {
            // If there is metrical hierarchy left in only one of the sections, find a match from the
            // highest stratum via start-time
            if !no_strata_left_a && no_strata_left_b {
                let set_idx = first_free_idx(&result);
                result[set_idx] = pair_higher_stratum_by_time(durations_a, durations_b, gnsm_a, idx_a_offset, idx_b_offset);
            } else if no_strata_left_a && !no_strata_left_b {
                let set_idx = first_free_idx(&result);
                let tmp = pair_higher_stratum_by_time(durations_b, durations_a, gnsm_b, idx_a_offset, idx_b_offset);
                result[set_idx] = (tmp.1, tmp.0);
            }
            // If there is metrical hierarchy left in both sections, match beats from the same stratum
            else {
                let set_idx = first_free_idx(&result);
                result[set_idx] = pair_highest_stratus(durations_a, durations_b, gnsm_a, gnsm_b, idx_a_offset, idx_b_offset);
            }
        }
    }

    // At this point, we should have some pairs in result
    result.iter().for_each(|(x, y)| assert!(x.is_some() || y.is_some()));

    // TODO sort by index... (shouldn't matter whether to sort by a or b, right??)
    // Call generate_interpolation_data_aux recursively on empty subsections, if there are any
    if !result.iter().all(|&(x, y)| x.is_some() && y.is_some()) {
        // TODO get unset passages
        // for each passage: recursive call and wiedereingliederung.
    }

    result
}

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
