use core::mem;

pub trait GroupRuns {
    type Item;

    fn group_runs<B>(&mut self, by: B) -> GroupedRuns<Self::Item, B>
    where B: FnMut(&Self::Item, &Self::Item) -> bool;
}

impl <I> GroupRuns for [I] {
    type Item = I;

    fn group_runs<B>(&mut self, by: B) -> GroupedRuns<'_, Self::Item, B>
    where B: FnMut(&Self::Item, &Self::Item) -> bool {
        GroupedRuns {slice: self, by}
    }
}

pub struct GroupedRuns<'a, I: 'a, B> {
    slice: &'a mut [I],
    by: B,
}

impl <'a, I: 'a, B> Iterator for GroupedRuns<'a, I, B>
where
    B: FnMut(&I, &I) -> bool
{
    type Item = &'a mut [I];

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }

        let mut i = 1;
        while i < self.slice.len() && (self.by)(&self.slice[i-1], &self.slice[i]) {
            i = i + 1;
        }
        let slice = mem::take(&mut self.slice);
        let (a, b): (Self::Item, Self::Item) = slice.split_at_mut(i);
        self.slice = b;
        Some(a)
    }
}
