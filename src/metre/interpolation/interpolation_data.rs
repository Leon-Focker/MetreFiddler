use serde::{Deserialize, Serialize};
use vizia_plug::vizia::prelude::Data;
use crate::metre::beat_origin::BeatOrigin;
use crate::metre::interpolation::index_pairs::IndexPairs;
use crate::util::{approx_eq, dry_wet, get_durations, get_start_times};

struct InterpolationDataHelper<'a> {
    durations: &'a[f32],
    starts: &'a[f32],
    gnsm: &'a[usize],
    len: usize,
    offset: usize,
}

/// Data determining how to interpolate between two Metres
/// Most important are the duration_pairs. This holds pairs of durations (one for each of two MetreDatas).
/// If one metric structure has more beats than the other, some of its beats will be paired with 0.0.
/// Additionally, a sorted list with all unique start-times is stored, plus a list of BeatOrigin,
/// telling us in which of the two Metres a start-time came from. These start-times correlate with the
/// stored interleaved_durations and interleaved_gnsm.
#[derive(Debug, Serialize, Deserialize, Clone, Data, Default)]
pub struct InterpolationData {
    duration_pairs: Vec<(f32, f32)>,
    interleaved_durations: Vec<f32>,
    interleaved_gnsm: Vec<usize>,

    unique_start_times: Vec<f32>,
    unique_start_time_origins: Vec<BeatOrigin>,
}

impl InterpolationData {

    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_durs_and_gnsm(durations_a: &[f32], durations_b: &[f32], gnsm_a: &[usize], gnsm_b: &[usize]) -> Self {
        assert_eq!(durations_a.len(), gnsm_a.len());
        assert_eq!(durations_b.len(), gnsm_b.len());

        Self::default()
            .set_duration_pairs(durations_a, durations_b, gnsm_a, gnsm_b)
            .set_interleaved_and_starts(durations_a, durations_b, gnsm_a, gnsm_b)
    }

    pub fn get_interpolated_durations(&self, interpolation: f32) -> impl Iterator<Item = f32> + '_ {
        self.duration_pairs
            .iter()
            .filter_map(move |&(a, b)| {
                let x = dry_wet(a, b, interpolation);
                (x > 0.0).then_some(x)
            })
    }

    pub fn interleaved_durations(&self) -> &[f32] {
        &self.interleaved_durations
    }

    pub fn unique_start_times(&self) -> &[f32] {
        &self.unique_start_times
    }

    pub fn unique_start_time_origins(&self) -> &[BeatOrigin] {
        &self.unique_start_time_origins
    }

    fn set_duration_pairs(mut self, durations_a: &[f32], durations_b: &[f32], gnsm_a: &[usize], gnsm_b: &[usize]) -> Self {
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

        self.duration_pairs = get_duration_pairs(data_a, data_b)
            .iter()
            .map(|&(idx_a, idx_b)| {
                (if let Some(idx) = idx_a {
                    *durations_a.get(idx).unwrap_or(&0.0)
                } else { 0.0 },
                 if let Some(idx) = idx_b {
                     *durations_b.get(idx).unwrap_or(&0.0)
                 } else { 0.0 })
            })
            .collect();

        self
    }

    fn set_interleaved_and_starts(mut self, durs_a: &[f32], durs_b: &[f32], gnsm_a: &[usize], gnsm_b: &[usize]) -> Self {
        let mut starts_a = get_start_times(&durs_a);
        let mut starts_b = get_start_times(&durs_b);
        let max_len = durs_a.len() + durs_b.len();

        starts_a.push(1.0);
        starts_b.push(1.0);

        starts_a.sort_by(|a, b| a.partial_cmp(b).unwrap());
        starts_b.sort_by(|a, b| a.partial_cmp(b).unwrap());

        starts_a.dedup_by(|a, b| approx_eq(*a, *b, 0.001));
        starts_b.dedup_by(|a, b| approx_eq(*a, *b, 0.001));

        let mut unique_start_times = Vec::with_capacity(max_len);
        let mut unique_start_time_origins = Vec::with_capacity(max_len);
        let mut interleaved_gnsm = Vec::with_capacity(max_len);

        let mut i = 0;
        let mut k = 0;

        while i < starts_a.len() || k < starts_b.len() {
            match (starts_a.get(i), starts_b.get(k)) {
                (Some(&a), Some(&b)) => {
                    if approx_eq(a, b, 0.001) {
                        unique_start_times.push(a);
                        unique_start_time_origins.push(BeatOrigin::Both);
                        interleaved_gnsm.push(gnsm_a[i % gnsm_a.len()].max(gnsm_b[k % gnsm_b.len()]));
                        i += 1;
                        k += 1;
                    } else if a < b {
                        unique_start_times.push(a);
                        unique_start_time_origins.push(BeatOrigin::MetreA);
                        interleaved_gnsm.push(gnsm_a[i % gnsm_a.len()]);
                        i += 1;
                    } else {
                        unique_start_times.push(b);
                        unique_start_time_origins.push(BeatOrigin::MetreB);
                        interleaved_gnsm.push(gnsm_b[k % gnsm_b.len()]);
                        k += 1;
                    }
                }
                (Some(&a), None) => {
                    unique_start_times.push(a);
                    unique_start_time_origins.push(BeatOrigin::MetreA);
                    interleaved_gnsm.push(gnsm_a[i % gnsm_a.len()]);
                    i += 1;
                }
                (None, Some(&b)) => {
                    unique_start_times.push(b);
                    unique_start_time_origins.push(BeatOrigin::MetreB);
                    interleaved_gnsm.push(gnsm_b[k % gnsm_b.len()]);
                    k += 1;
                }
                (None, None) => break,
            }
        }

        let interleaved_durations = get_durations(&unique_start_times).collect::<Vec<_>>();

        self.unique_start_times = unique_start_times;
        self.unique_start_time_origins = unique_start_time_origins;
        self.interleaved_durations = interleaved_durations;
        self.interleaved_gnsm = interleaved_gnsm;

        self
    }
}



/// Given durations A and B, look for identical start times. For each identical start time in both
/// sets of durations, get their indices and pair them into result.
fn pair_identical_start_times(result: &mut IndexPairs, data_a: &InterpolationDataHelper, data_b: &InterpolationDataHelper) {
    for (i, &x) in data_a.starts.iter().enumerate() {
        if let Some(pos) = data_b.starts.iter().position(|&y| approx_eq(x, y, 0.001)) {
            result.set_first_free((Some(i + data_a.offset), Some(pos + data_b.offset)))
        }
    }
}

/// While durations A does have some metrical hierarchy indicated by gnsm_a, durations B does not.
/// Find the beat with the highest metrical value in durations A and pair it with the closest beat from B by start-time
fn pair_higher_stratum_by_time(data_a: &InterpolationDataHelper, data_b: &InterpolationDataHelper)  -> (Option<usize>, Option<usize>) {
    let no_strata_left_b = data_b.gnsm.iter().all(|&x| x == *data_b.gnsm.get(0).unwrap_or(&0));
    assert!(no_strata_left_b);
    // find the indices which belong to the highest stratum
    let highest_stratum = *data_a.gnsm.iter().max().unwrap_or(&1);
    let idx_a = data_a.gnsm.iter().rposition(|&x| x == highest_stratum).unwrap_or(data_a.gnsm.len() -1);
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

/// Pair the beats with the highest metrical value from each set of durations.
fn pair_highest_stratus (data_a: &InterpolationDataHelper, data_b: &InterpolationDataHelper) -> (Option<usize>, Option<usize>){
    // find the indices which belong to the highest stratus
    let highest_stratum_a = *data_a.gnsm.iter().max().unwrap_or(&1);
    let idx_a = data_a.gnsm.iter().rposition(|&x| x == highest_stratum_a).unwrap_or(data_a.gnsm.len() -1);
    let highest_stratum_b = *data_b.gnsm.iter().max().unwrap_or(&1);
    let idx_b = data_b.gnsm.iter().rposition(|&x| x == highest_stratum_b).unwrap_or(data_b.gnsm.len() -1);

    (Some(idx_a + data_a.offset), Some(idx_b + data_b.offset))
}

/// Return a vector of pairs of indices.
fn get_duration_pairs(data_a: InterpolationDataHelper, data_b: InterpolationDataHelper) -> IndexPairs {
    let max_len = data_a.len.max(data_b.len);
    let no_strata_left_a = data_a.gnsm.iter().all(|&x| x == *data_a.gnsm.get(0).unwrap_or(&0));
    let no_strata_left_b = data_b.gnsm.iter().all(|&x| x == *data_b.gnsm.get(0).unwrap_or(&0));
    let mut result = IndexPairs::new_with_len(max_len);

    // Apply one of the methods below (either complete result or match some indices),
    // then call recursively with empty subsections

    // If both sections are of the same length, one sections is empty, or when all are of the same stratum:
    if data_a.len == data_b.len
        || data_a.durations.is_empty()
        || data_b.durations.is_empty()
        || (no_strata_left_a && no_strata_left_b) {
        result.ascending_indices_with_padding(max_len, data_a.len, data_b.len, data_a.offset, data_b.offset);
    } else {
        // try finding pairs via similar start-times, only try this once (when offsets = 0), because else the first will always match
        if data_a.offset == 0 && data_b.offset == 0 {
            pair_identical_start_times(&mut result, &data_a, &data_b);
        }
        // If difference in length is just 1, append 0.0, else look for a more complicated method to match some pairs
        else if result.all_free() &&
            data_a.len.abs_diff(data_b.len) == 1 {
            result.ascending_indices_with_padding(max_len, data_a.len, data_b.len, data_a.offset, data_b.offset);
        } else {
            // If there is metrical hierarchy left in only one of the sections, find a match from the
            // highest stratum via start-time
            if !no_strata_left_a && no_strata_left_b {
                result.set_first_free(pair_higher_stratum_by_time(&data_a, &data_b));
            } else if no_strata_left_a && !no_strata_left_b {
                let (tmp_b, tmp_a) = pair_higher_stratum_by_time(&data_b, &data_a);
                result.set_first_free((tmp_a, tmp_b));
            }
            // If there is metrical hierarchy left in both sections, match beats from the same stratum
            else {
                result.set_first_free(pair_highest_stratus(&data_a, &data_b));
            }
        }
    }

    // At this point, we should have some pairs in result
    assert!(result.iter().any(|&(x, y)| x.is_some() || y.is_some()));

    result.sort();

    // TODO I'm sure this could be more concise...
    // Check which subsections have yet to be set, call this function recursively on them
    let mut last_a = data_a.offset;
    let mut last_b = data_b.offset;
    let mut flag = false;
    let mut subseqs: IndexPairs = IndexPairs::default();
    for (x, y) in result.iter() {
        if let (Some(a), Some(b)) = (x, y) {
            let diff_a = a - last_a;
            let diff_b = b - last_b;

            if diff_a.max(diff_b) > if flag { 1 } else { 0 } {
                subseqs.append(&mut call_with_slices(&data_a, &data_b,
                                                     if flag { last_a + 1} else { last_a } - data_a.offset,
                                                     if flag { last_b + 1} else { last_b } - data_b.offset,
                                                     if flag { diff_a - 1 } else { diff_a },
                                                     if flag { diff_b - 1 } else { diff_b },));
            }

            last_a = *a;
            last_b = *b;
            flag = true;
        }
        if let (Some(_), None) = (x, y) {
            last_a += 1;
        }
        if let (None, Some(_)) = (x, y) {
            last_b += 1;
        }
    }

    let diff_a = data_a.len + data_a.offset - last_a;
    let diff_b = data_b.len + data_b.offset - last_b;

    if diff_a.max(diff_b) > if flag { 1 } else { 0 } {
        subseqs.append(&mut call_with_slices(&data_a, &data_b,
                                             if flag { last_a + 1} else { last_a } - data_a.offset,
                                             if flag { last_b + 1} else { last_b } - data_b.offset,
                                             if flag { diff_a - 1 } else { diff_a },
                                             if flag { diff_b - 1 } else { diff_b },));
    }

    // Add the subsections to result
    subseqs.reverse();
    for elem in result.iter_mut() {
        if let (None, None) = *elem {
            *elem = subseqs.pop().unwrap_or((None, None));
        }
    }

    result.sort();
    result
}

fn call_with_slices(data_a: &InterpolationDataHelper, data_b: &InterpolationDataHelper, start_a: usize, start_b: usize, len_a: usize, len_b: usize) -> IndexPairs {
    let end_a = start_a + len_a;
    let new_data_a = InterpolationDataHelper {
        durations: &data_a.durations[start_a..end_a],
        starts: &data_a.starts[start_a..end_a],
        gnsm: &data_a.gnsm[start_a..end_a],
        len: len_a,
        offset: start_a + data_a.offset,
    };
    let end_b = start_b + len_b;
    let new_data_b = InterpolationDataHelper {
        durations: &data_b.durations[start_b..end_b],
        starts: &data_b.starts[start_b..end_b],
        gnsm: &data_b.gnsm[start_b..end_b],
        len: len_b,
        offset: start_b + data_b.offset,
    };
    get_duration_pairs(new_data_a, new_data_b)
}