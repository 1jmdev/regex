use crate::matcher;

/// Storage for capture-group `(start, end)` pairs.
///
/// Small regexes (up to 3 groups including group 0) use the inline array to
/// avoid heap allocation. Larger regexes fall back to a `Vec`.
#[derive(Clone, Debug)]
pub enum Slots {
    /// Inline storage for patterns with at most 3 capture groups.
    Inline3([Option<(usize, usize)>; 3]),
    /// Heap-allocated storage for patterns with more than 3 capture groups.
    Heap(matcher::Slots),
}

impl Slots {
    /// Returns the `(start, end)` pair for group `i`, or `None` if absent.
    #[inline(always)]
    pub fn get(&self, i: usize) -> Option<(usize, usize)> {
        match self {
            Slots::Inline3(slots) => slots.get(i).copied().flatten(),
            Slots::Heap(slots) => slots.get(i).copied().flatten(),
        }
    }

    /// Returns the total number of slots (including group 0).
    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            Slots::Inline3(slots) => slots.len(),
            Slots::Heap(slots) => slots.len(),
        }
    }
}
