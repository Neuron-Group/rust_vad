use std::collections::VecDeque;

use crate::type_trait::*;

pub struct FixedLengthQueue<T> {
    pub queue: VecDeque<T>,
    length: usize,
}

impl<T> FixedLengthQueue<T> {
    pub fn new(size: usize) -> Self {
        assert!(size > 0);
        Self {
            queue: VecDeque::with_capacity(size),
            length: size,
        }
    }

    pub fn push_front_(&mut self, value: T) {
        if self.queue.len() >= self.length {
            self.queue.pop_back();
        }

        self.queue.push_front(value);
    }

    pub fn push_front(mut self, value: T) -> Self {
        Self {
            length: self.length,
            queue: {
                if self.queue.len() >= self.length {
                    self.queue.pop_back();
                }

                self.queue.push_front(value);
                self.queue
            },
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn size(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl<T: IntTrait> FixedLengthQueue<T> {
    pub fn mean_for_int<TOutput: FloatTrait + From<T>>(&self) -> Option<TOutput> {
        if self.is_empty() {
            return <TOutput as num_traits::NumCast>::from(0.0);
        }

        let sum: T = self.queue.iter().cloned().sum();

        let cnt = self.length;

        let sum_float = <TOutput as num_traits::NumCast>::from(sum)?;
        let count_float = <TOutput as num_traits::NumCast>::from(cnt)?;
        Some(sum_float / count_float)
    }
}

impl<T: FloatTrait> FixedLengthQueue<T> {
    pub fn mean(&self) -> Option<T> {
        match self.is_empty() {
            true => Some(<T as num_traits::NumCast>::from(0.0)?),
            false => {
                let sum: T = self.queue.iter().cloned().sum();
                let count = <T>::from(self.len())?;
                Some(sum / count)
            }
        }
    }
}
