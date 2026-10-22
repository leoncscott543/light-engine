use std::collections::VecDeque;

/// Fixed-capacity rolling window of samples for live monitoring.
#[derive(Debug, Clone)]
pub struct Series<T = f64> {
    capacity: usize,
    samples: VecDeque<T>,
}

impl<T: Copy> Series<T> {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "series capacity must be non-zero");
        Self {
            capacity,
            samples: VecDeque::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, sample: T) {
        if self.samples.len() == self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn latest(&self) -> Option<T> {
        self.samples.back().copied()
    }

    /// Oldest to newest.
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = T> + ExactSizeIterator + '_ {
        self.samples.iter().copied()
    }
}

impl Series<f64> {
    pub fn min_max(&self) -> Option<(f64, f64)> {
        self.iter().fold(None, |acc, v| match acc {
            None => Some((v, v)),
            Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
        })
    }

    pub fn mean(&self) -> Option<f64> {
        (!self.is_empty()).then(|| self.iter().sum::<f64>() / self.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_oldest_at_capacity() {
        let mut s = Series::new(3);
        for v in 1..=5 {
            s.push(v as f64);
        }
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![3.0, 4.0, 5.0]);
        assert_eq!(s.latest(), Some(5.0));
        assert_eq!(s.min_max(), Some((3.0, 5.0)));
        assert_eq!(s.mean(), Some(4.0));
    }

    #[test]
    fn empty_series() {
        let s: Series = Series::new(4);
        assert_eq!(s.min_max(), None);
        assert_eq!(s.mean(), None);
    }
}
