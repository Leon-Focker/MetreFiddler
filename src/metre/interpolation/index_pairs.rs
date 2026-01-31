use std::cmp::Ordering;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Default)]
pub struct IndexPairs {
    data: Vec<(Option<usize>, Option<usize>)>,
}

impl IndexPairs {
    pub(crate) fn with_len(len: usize) -> Self {
        IndexPairs {
            data: vec![(None, None); len],
        }
    }
    /// Collect ascending indices from range of each section, choose out-of-bounds index if necessary.
    pub(crate) fn ascending_indices_with_padding(len: usize, len_a: usize, len_b: usize, offset_a: usize, offset_b: usize) -> Self {
        IndexPairs {
            data:   (0..len).map(|i| (
                if i < len_a {
                    Some(i + offset_a)
                } else { None },
                if i < len_b {
                    Some(i + offset_b)
                } else { None },
            )).collect()
        }
    }
    pub(crate) fn set_first_free(&mut self, value: (Option<usize>, Option<usize>)) {
        if let Some(elem) = self
            .iter_mut()
            .find(|(x, y)| x.is_none() && y.is_none())
        {
            *elem = value
        }
    }
    pub(crate) fn all_free(&self) -> bool {
        self.iter().all(|&(x, y)| x.is_none() && y.is_none())
    }
    pub(crate) fn sort(&mut self) {
        self.data.sort_by(|&(a, b), &(x, y)|
            if let (Some(a), Some(x)) = (a, x) {
                a.cmp(&x)
            } else if let (Some(b), Some(y)) = (b, y) {
                b.cmp(&y)
            } else { Ordering::Equal} );
    }
}

impl Deref for IndexPairs {
    type Target = Vec<(Option<usize>, Option<usize>)>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for IndexPairs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}