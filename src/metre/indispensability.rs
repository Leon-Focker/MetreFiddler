use crate::metre::rqq::RQQ;

/// Get the indispensability values for each pulse/beat in a stratified meter,
/// according to Clarence Barlow and Bernd Härpfer. However, here the
/// indispensability values are inverted, so that the most important beat is
/// always 0!
#[allow(dead_code)]
pub fn rqq_to_indispensability_list(rqq: RQQ) -> Result<Vec<usize>, String> {
    gnsm_to_indispensability_list(&rqq.to_gnsm()?)
}

/// Get the indispensability values for each pulse/beat in a stratified meter,
/// according to Clarence Barlow and Bernd Härpfer. However, here the
/// indispensability values are inverted, so that the most important beat is
/// always 0!
pub fn gnsm_to_indispensability_list(gnsm: &[usize]) -> Result<Vec<usize>, String> {
    let len = gnsm.len();
    let mut result: Vec<isize> = vec![-1; len];
    let mut indices: Vec<usize> = vec![];
    let mut set_indices: Vec<usize> = vec![];
    let mut remaining_indices: Vec<usize> = vec![];
    let mut layer: isize = *gnsm.iter().max().unwrap() as isize;

    get_indices(layer, &gnsm, &mut indices);
    let mut old_indices = indices.clone();

    for (e, i) in fundamental_indispensability(indices.len())
        .iter()
        .zip(indices.iter()) {
        result[*i] = *e as isize;
    }

    while layer >= 0 {

        remaining_indices.clear();
        for i in indices.iter() {
            if result[*i] < 0 { remaining_indices.push(*i); }
        }

        if remaining_indices.is_empty() {
            layer -= 1;
            get_indices(layer, &gnsm, &mut indices);
            copy_from_neighbours(&indices, &mut set_indices, &mut result, len);
        } else {
            copy_from_neighbours(&remaining_indices, &mut set_indices, &mut result, len);
        }

        for (val, idx) in sort_copied_indices(&result, &set_indices).iter().enumerate() {
            result[*idx] = val as isize;
        }

        for i in old_indices.iter() {
            result[*i] += set_indices.len() as isize;
        }

        old_indices.extend_from_slice(&set_indices);
    }

    // invert values
    let max = *result.iter().max().unwrap();
    Ok(result.iter().map(|x| (max - *x) as usize).collect())
}

// helper functions
fn get_indices(layer: isize, gnsm: &[usize], indices: &mut Vec<usize>) {
    indices.clear();
    for (i, e) in gnsm.iter().enumerate() {
        if *e == layer as usize {
            indices.push(i);
        }
    }
}

fn fundamental_indispensability(len: usize) -> Vec<usize> {
    let mut result = Vec::with_capacity(len);
    if len > 0 { result.push(len-1) }
    if len > 1 {
        for i in 0..len-1 {
            result.push(i+1);
        }
    }
    result
}

fn next_set_index(result: &[isize], mut idx: usize, len: usize) -> usize {
    loop {
        idx = (idx + 1).rem_euclid(len);
        if result[idx] >= 0 { return idx; }
    }
}

fn copy_from_neighbours(indices: &[usize], set_indices: &mut Vec<usize>, result: &mut [isize], len: usize) -> () {
    set_indices.clear();
    for i in 0..indices.len() {
        let idx = indices[i];
        let next = next_set_index(result, idx, len);
        if i+1 == indices.len() || (idx < next && next < indices[i+1]) {
            set_indices.push(idx);
            result[idx] = result[next];
        }
    }
}

fn sort_copied_indices(result: &[isize], set_indices: &[usize]) -> Vec<usize> {
    let mut order: Vec<usize> = Vec::with_capacity(set_indices.len());
    let mut n = 0;
    while order.len() < set_indices.len() {
        for &i in set_indices {
            if n == result[i] { order.push(i) }
        }
        n += 1;
    }
    order
}

