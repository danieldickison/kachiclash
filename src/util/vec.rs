use core::mem;

pub trait GroupRuns {
    type Item;

    fn group_runs<B>(&self, by: B) -> GroupedRuns<'_, Self::Item, B>
    where
        B: FnMut(&Self::Item, &Self::Item) -> bool;

    fn group_runs_mut<B>(&mut self, by: B) -> GroupedRunsMut<'_, Self::Item, B>
    where
        B: FnMut(&Self::Item, &Self::Item) -> bool;
}

impl<I> GroupRuns for [I] {
    type Item = I;

    fn group_runs<B>(&self, by: B) -> GroupedRuns<'_, Self::Item, B>
    where
        B: FnMut(&Self::Item, &Self::Item) -> bool,
    {
        GroupedRuns { slice: self, by }
    }

    fn group_runs_mut<B>(&mut self, by: B) -> GroupedRunsMut<'_, Self::Item, B>
    where
        B: FnMut(&Self::Item, &Self::Item) -> bool,
    {
        GroupedRunsMut { slice: self, by }
    }
}

pub struct GroupedRuns<'a, I: 'a, B> {
    slice: &'a [I],
    by: B,
}

pub struct GroupedRunsMut<'a, I: 'a, B> {
    slice: &'a mut [I],
    by: B,
}

impl<'a, I: 'a, B> Iterator for GroupedRuns<'a, I, B>
where
    B: FnMut(&I, &I) -> bool,
{
    type Item = &'a [I];

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }

        let mut i = 1;
        while i < self.slice.len() && (self.by)(&self.slice[i - 1], &self.slice[i]) {
            i += 1;
        }
        let slice = mem::take(&mut self.slice);
        let (a, b): (Self::Item, Self::Item) = slice.split_at(i);
        self.slice = b;
        Some(a)
    }
}

impl<'a, I: 'a, B> Iterator for GroupedRunsMut<'a, I, B>
where
    B: FnMut(&I, &I) -> bool,
{
    type Item = &'a mut [I];

    fn next(&mut self) -> Option<Self::Item> {
        if self.slice.is_empty() {
            return None;
        }

        let mut i = 1;
        while i < self.slice.len() && (self.by)(&self.slice[i - 1], &self.slice[i]) {
            i += 1;
        }
        let slice = mem::take(&mut self.slice);
        let (a, b): (Self::Item, Self::Item) = slice.split_at_mut(i);
        self.slice = b;
        Some(a)
    }
}
