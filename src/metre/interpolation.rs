use std::cmp::Ordering;
use nih_plug::{nih_dbg, nih_log};
use serde::{Deserialize, Serialize};
use vizia_plug::vizia::prelude::Data;
use crate::util::{approx_eq, get_start_times};

// TODO works for simple metrical hierarchies, test for more complex cases!
// TODO This file is kind of a mess!

/// Holds pairs of durations (one for each of two MetreDatas). If one metric structure has more
/// beats than the other, some of its beats will be paired with 0.0.
#[derive(Debug, Serialize, Deserialize, Clone, Data)]
pub struct InterpolationData {
    pub value: Vec<(f32, f32)>,
}

struct InterpolationDataHelper<'a> {
    durations: &'a[f32],
    starts: &'a[f32],
    gnsm: &'a[usize],
    len: usize,
    offset: usize,
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
    let data_a = InterpolationDataHelper {
        durations: durations_a,
        starts: &get_start_times(durations_a),
        gnsm: gnsm_a,
        len: durations_a.len(),
        offset: 0
    };
    let data_b = InterpolationDataHelper {
        durations: durations_b,
        starts: &get_start_times(durations_b),
        gnsm: gnsm_b,
        len: durations_b.len(),
        offset: 0
    };
    InterpolationData {
        // Get pairs of indices and map them to the actual durations from A and B.
        value: generate_interpolation_data_aux(data_a, data_b)
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
fn ascending_indices_with_padding(len: usize, len_a: usize, len_b: usize, offset_a: usize, offset_b: usize) -> Vec<(Option<usize>, Option<usize>)> {
    (0..len).map(|i| (
        if i < len_a {
            Some(i + offset_a)
        } else { None },
        if i < len_b {
            Some(i + offset_b)
        } else { None },
    )).collect()
}

/// Given durations A and B, look for identical start times. For each identical start time in both
/// sets of durations, get their indices and pair them into result.
fn pair_identical_start_times(result: &mut [(Option<usize>, Option<usize>)], data_a: &InterpolationDataHelper, data_b: &InterpolationDataHelper) {
    for (i, &x) in data_a.starts.iter().enumerate() {
        if let Some(pos) = data_b.starts.iter().position(|&y| approx_eq(x, y, 0.001)) {
            result[first_free_idx(result)]
                = (Some(i + data_a.offset), Some(pos + data_b.offset))
        }
    }
}

/// While durations A does have some metrical hierarchy indicated by gnsm_a, durations B does not.
/// Find the beat with the highest metrical value in durations A and pair it with the closest beat from B by start-time
fn pair_higher_stratum_by_time(data_a: &InterpolationDataHelper, data_b: &InterpolationDataHelper)  -> (Option<usize>, Option<usize>) {
    // find the indices which belong to the highest stratum
    let highest_stratum = *data_a.gnsm.iter().max().unwrap_or(&1);
    let idx_a = data_a.gnsm.iter().rposition(|&x| x == highest_stratum).expect("something went wrong in fn pair_higher_stratum_by_time");
    let start_time_a = data_a.starts[idx_a];
    // get index for Start in B that's closest to start_time_a
    let idx_b = data_b.starts.iter()
        .map(| &start| (start - start_time_a).abs())
        .enumerate()
        .min_by(| (_, x), (_, y) |x.total_cmp(y))
        .unwrap_or((0, 0.0))
        .0;

    (Some(idx_a + data_a.offset), Some(idx_b + data_b.offset))
}

fn pair_highest_stratus (data_a: &InterpolationDataHelper, data_b: &InterpolationDataHelper) -> (Option<usize>, Option<usize>) {
    // find the indices which belong to the highest stratus
    let highest_stratum_a = *data_a.gnsm.iter().max().unwrap_or(&1);
    let idx_a = data_a.gnsm.iter().rposition(|&x| x == highest_stratum_a).expect("something went wrong in fn pair_highest_stratus ");
    let highest_stratum_b = *data_b.gnsm.iter().max().unwrap_or(&1);
    let idx_b = data_b.gnsm.iter().rposition(|&x| x == highest_stratum_b).expect("something went wrong in fn pair_highest_stratus ");

    (Some(idx_a + data_a.offset), Some(idx_b + data_b.offset))
}

/// Return a vector of pairs of indices.
fn generate_interpolation_data_aux(data_a: InterpolationDataHelper, data_b: InterpolationDataHelper) -> Vec<(Option<usize>, Option<usize>)> {
    nih_dbg!("Tracing starts here!:");
    let max_len = data_a.len.max(data_b.len);
    let no_strata_left_a = data_a.gnsm.iter().all(|&x| x == *data_a.gnsm.get(0).unwrap_or(&0));
    let no_strata_left_b = data_b.gnsm.iter().all(|&x| x == *data_a.gnsm.get(0).unwrap_or(&0));
    let mut result = vec![(None, None); max_len];

    nih_log!("dur_a: {:?}", &data_a.durations);
    nih_log!("dur_b: {:?}", &data_b.durations);

    // Apply one of the methods below (either complete result or match some indices),
    // then call recursively with empty subsections

    // If both sections are of the same length, one sections is empty, or when all are of the same stratum:
    if data_a.len == data_b.len
        || data_a.durations.is_empty()
        || data_b.durations.is_empty()
        || (no_strata_left_a && no_strata_left_b) {
        nih_dbg!("simple!");
        result = ascending_indices_with_padding(max_len, data_a.len, data_b.len, data_a.offset, data_b.offset);
    } else {
        // try finding pairs via similar start-times, only try this once (when offsets = 0), because else the first will always match
        if data_a.offset == 0 && data_b.offset == 0 {
            nih_dbg!("start times");
            pair_identical_start_times(&mut result, &data_a, &data_b);
        }
        // If difference in length is just 1, append 0.0, else look for a more complicated method to match some pairs
        else if all_free(&result) &&
            data_a.len.abs_diff(data_b.len) == 1 {
            nih_dbg!("difference of 1");
            result = ascending_indices_with_padding(max_len, data_a.len, data_b.len, data_a.offset, data_b.offset);
        } else {
            // If there is metrical hierarchy left in only one of the sections, find a match from the
            // highest stratum via start-time
            if !no_strata_left_a && no_strata_left_b {
                nih_dbg!("option A:");
                let set_idx = first_free_idx(&result);
                result[set_idx] = pair_higher_stratum_by_time(&data_a, &data_b);
            } else if no_strata_left_a && !no_strata_left_b {
                nih_dbg!("Option B:");
                let set_idx = first_free_idx(&result);
                let tmp = pair_higher_stratum_by_time(&data_a, &data_b);
                result[set_idx] = (tmp.1, tmp.0);
            }
            // If there is metrical hierarchy left in both sections, match beats from the same stratum
            else {
                nih_dbg!("Option C:");
                let set_idx = first_free_idx(&result);
                result[set_idx] = pair_highest_stratus(&data_a, &data_b);
            }
        }
    }

    // At this point, we should have some pairs in result
    assert!(result.iter().any(|&(x, y)| x.is_some() || y.is_some()));

    result.sort_by(|&(a, b), &(x, y)|
        if let (Some(a), Some(x)) = (a, x) {
            a.cmp(&x)
        } else if let (Some(b), Some(y)) = (b, y) {
            b.cmp(&y)
        } else { Ordering::Equal} );

    nih_log!("sorted: {:?}", &result);

    let mut last_a = data_a.offset;
    let mut last_b = data_b.offset;
    let mut subseqs = Vec::new();
    for (x, y) in result.iter() {
        if let (Some(a), Some(b)) = (x, y) {
            if let Some(mut subseq) = recursive_call(&data_a, &data_b, *a, last_a, *b, last_b) {
                subseqs.append(&mut subseq)
            }

            last_a = *a;
            last_b = *b;
        }
    }

    if let Some(mut subseq) = recursive_call(&data_a, &data_b, data_a.len, last_a, data_b.len, last_b) {
        subseqs.append(&mut subseq)
    }

    subseqs.reverse();
    for elem in result.iter_mut() {
        if let (None, None) = *elem {
            *elem = subseqs.pop().unwrap_or((None, None));
        }
    }

    // TODO by now result should have its own type and methods
    result.sort_by(|&(a, b), &(x, y)|
        if let (Some(a), Some(x)) = (a, x) {
            a.cmp(&x)
        } else if let (Some(b), Some(y)) = (b, y) {
            b.cmp(&y)
        } else { Ordering::Equal} );

    nih_log!("the data: {:?}", &result);

    result
}

fn recursive_call(data_a: &InterpolationDataHelper, data_b: &InterpolationDataHelper, a: usize, last_a: usize, b: usize, last_b: usize) -> Option<Vec<(Option<usize>, Option<usize>)>> {

    nih_dbg!("Recursive call:");

    let diff_a = a.saturating_sub(last_a);
    let diff_b = b.saturating_sub(last_b);

    if diff_a > 1 || diff_b > 1 {
        let new_data_a = InterpolationDataHelper {
            durations: &data_a.durations[last_a+1..a],
            starts: &data_a.starts[last_a+1..a],
            gnsm: &data_a.gnsm[last_a+1..a],
            len: diff_a - 1,
            offset: last_a+1,
        };
        let new_data_b = InterpolationDataHelper {
            durations: &data_b.durations[last_b+1..b],
            starts: &data_b.starts[last_b+1..b],
            gnsm: &data_b.gnsm[last_b+1..b],
            len: diff_b - 1,
            offset: last_b+1,
        };
        Some(generate_interpolation_data_aux(new_data_a, new_data_b))
    } else { None }
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
