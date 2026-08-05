use crate::collection::Queue;

pub struct LazyGrowList<T>
{
    data:         Vec<T>,
    has_values:   Vec<bool>,
    free_indices: Queue<usize>,
}
impl<T> Default for LazyGrowList<T>
{
    fn default() -> Self
    {
        Self::new()
    }
}
impl<T> LazyGrowList<T>
{
    pub fn new() -> Self
    {
        Self {
            data:         Vec::new(),
            free_indices: Queue::new(),
            has_values:   Vec::new(),
        }
    }

    pub fn push(&mut self, item: T) -> usize
    {
        if let Some(free_index) = self.free_indices.dequeue()
        {
            self.data[free_index] = item;
            self.has_values[free_index] = true;
            free_index
        }
        else
        {
            self.data.push(item);
            self.has_values.push(true);
            self.data.len() - 1
        }
    }

    pub fn remove(&mut self, index: usize)
    {
        debug_assert!(index < self.data.len());
        debug_assert!(self.has_values[index], "index {index} is already free");
        self.free_indices.enqueue(index);
        self.has_values[index] = false;
    }

    pub fn get(&self, index: usize) -> Option<&T>
    {
        if index < self.data.len() && self.has_values[index]
        {
            Some(unsafe { self.data.get_unchecked(index) })
        }
        else
        {
            None
        }
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T>
    {
        if index < self.data.len() && self.has_values[index]
        {
            Some(unsafe { self.data.get_unchecked_mut(index) })
        }
        else
        {
            None
        }
    }
}
