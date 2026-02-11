use std::fmt::Debug;
use std::iter::Sum;
use num_traits::{Float, Num, NumCast};

///  Given a value within an original range, return its value within a new range.
///
/// # Examples
/// ```
/// let rescaled = metrefiddler::rescale(0.5, 0.0, 1.0, 0.0, 100.0, false).unwrap();
/// 
/// assert_eq!(rescaled, 50.0);
/// ```
pub fn rescale<T: Num + PartialOrd + Copy + Debug>(
    val: T,
    min: T,
    max: T,
    new_min: T,
    new_max: T,
    clamp_out_of_range: bool,
) -> Result<T, &'static str> {
    if min >= max || new_min >= new_max { 
        return Err("rescale: min must be < max and new_min < new_max!")
    };

    let mut val = val;

    if val < min || val > max {
        if clamp_out_of_range {
            val = if val <= min { 
                min                
            } else if val >= max { 
                max
            } else {
                val
            };
        } else {
            return Err("rescale: value out of range!")
        }
    }

    let range1 = max - min;
    let range2 = new_max - new_min;
    let prop = (val - min) / range1;
    Ok(new_min + prop * range2)
}

/// Given a selector between 0.0 and 1.0 and a list of weights, return the index 
/// corresponding to the position in the cumulative distribution defined by the weights.
/// Returns an error if weights are empty or rescaling fails.
/// 
/// # Examples
/// ```
/// let elements = vec!['a', 'b', 'c', 'd', 'e', 'f', 'g'];
/// let weights = vec![1.0, 1.0, 2.0, 2.0, 3.0, 1.0, 2.0];
/// let index = metrefiddler::decider(0.1, &weights).unwrap();
/// let element = elements[index as usize];
///
/// assert_eq!(element, 'b');
/// ```
pub fn decider<T: Num + PartialOrd + Copy + Debug + Sum<T>>(selector: T, weights: &[T]) -> Result<T, &'static str> {
    let selector: T = rescale(selector, T::zero(), T::one(), T::zero(), weights.iter().copied().sum(), true)?;
    decider_aux(selector, &weights[1..], T::zero(), weights[0])
}

fn decider_aux<T: Num + PartialOrd + Copy + Debug>(selector: T, ls1: &[T], index: T, sum: T) -> Result<T, &'static str> {
    if ls1.is_empty() || selector < sum {
        Ok(index)
    } else {
        decider_aux(selector, &ls1[1..], index + T::one(), sum + ls1[0])
    }
}

pub fn dry_wet<T: NumCast + Copy>(dry: T, wet: T, mix: f32) -> f32 {
    let mix = mix.clamp(0.0, 1.0);

    let dry = NumCast::from(dry).unwrap_or(0.0_f32);
    let wet = NumCast::from(wet).unwrap_or(0.0_f32);

    dry * (1.0 - mix) + wet * mix
}

pub fn _interpolate_vectors<T: NumCast + Copy + num_traits::Zero>(vec_a: &[T], vec_b: &[T], interpolation: f32) -> Vec<f32> {
    let max_len = vec_a.len().max(vec_b.len());
    let mut result = Vec::with_capacity(max_len);

    for (i, element) in result.iter_mut().enumerate().take(max_len) {
        *element = dry_wet(*vec_a.get(i).unwrap_or(&T::zero()), *vec_b.get(i).unwrap_or(&T::zero()), interpolation);
    }

    result
}

pub fn get_start_times<T: Num + Copy>(durations: &[T]) -> Vec<T> {
    let mut time = T::zero();
    let mut result = Vec::with_capacity(durations.len());

    for &dur in durations {
        result.push(time);
        time = dur + time;
    }

    result
}

pub fn get_durations<T: Num + Copy>(start_times: &[T]) -> Vec<T> {
    let mut last = start_times[0];
    start_times[1..]
        .iter()
        .map(|&start| {
            let dur = start - last;
            last = start;
            dur
        }).collect()
}

pub fn approx_eq<T: Float>(a: T, b: T, epsilon: T) -> bool {
    (a - b).abs() <= epsilon
}
