use std::collections::VecDeque;

#[derive(Default, Debug)]
pub struct Queue<T>
{
    queue: VecDeque<T>,
}

impl<T> Queue<T>
{
    #[inline]
    pub fn new() -> Self
    {
        Self { queue: VecDeque::new() }
    }
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self
    {
        Self {
            queue: VecDeque::with_capacity(capacity),
        }
    }

    #[inline]
    pub fn len(&self) -> usize
    {
        self.queue.len()
    }
    #[inline]
    pub fn is_empty(&self) -> bool
    {
        self.queue.is_empty()
    }
    #[inline]
    pub fn enqueue(&mut self, val: T)
    {
        self.queue.push_back(val);
    }
    #[inline]
    pub fn dequeue(&mut self) -> Option<T>
    {
        self.queue.pop_front()
    }
    #[inline]
    pub fn enqueue_batch<I>(&mut self, vals: I)
    where I: IntoIterator<Item = T>
    {
        self.queue.extend(vals);
    }
    #[inline]
    pub fn dequeue_batch(&mut self, count: usize, output: &mut Vec<T>) -> usize
    {
        let count = count.min(self.queue.len());
        output.extend(self.queue.drain(..count));
        count
    }
    #[inline]
    pub fn peek(&self) -> Option<&T>
    {
        self.queue.front()
    }
}
