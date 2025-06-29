use std::fmt::Debug;
use std::iter::Sum;
use num_traits::Num;

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
            } else { val };
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
pub fn decider<T: Num + PartialOrd + Copy + Debug + Sum<T>>(selector: T, weights: &[T]) -> Result<T, String> {
    let selector: T = rescale(selector, T::zero(), T::one(), T::zero(), weights.iter().copied().sum(), false)?;
    decider_aux(selector, &weights[1..], T::zero(), weights[0])
}

fn decider_aux<T: Num + PartialOrd + Copy + Debug>(selector: T, ls1: &[T], index: T, sum: T) -> Result<T, String> {
    if ls1.is_empty() {
        Ok(index)
    } else if selector < sum {
        return Ok(index)
    } else {
        decider_aux(selector, &ls1[1..], index + T::one(), sum + ls1[0])
    }
}