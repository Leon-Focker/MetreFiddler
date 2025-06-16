use std::fmt::Debug;
use std::iter::Sum;
use num_traits::Num;

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