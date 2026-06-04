use crate::matcher;

#[derive(Clone, Debug)]
pub enum Slots {
    Inline3([Option<(usize, usize)>; 3]),
    Heap(matcher::Slots),
}

impl Slots {
    #[inline(always)]
    pub fn get(&self, i: usize) -> Option<(usize, usize)> {
        match self {
            Slots::Inline3(slots) => slots.get(i).copied().flatten(),
            Slots::Heap(slots) => slots.get(i).copied().flatten(),
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            Slots::Inline3(slots) => slots.len(),
            Slots::Heap(slots) => slots.len(),
        }
    }
}
