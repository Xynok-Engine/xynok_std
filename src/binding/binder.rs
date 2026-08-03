use std::sync::{Arc, Mutex};

type Listener<T> = Box<dyn Fn(&T) + Send + 'static>;

struct ListenerMetaData<T>
{
    pub(crate) listener:       Listener<T>,
    pub(crate) received_event: bool,
}

struct BinderInner<T>
{
    data:      T,
    listeners: Vec<(usize, ListenerMetaData<T>)>,
    next_id:   usize,
}

impl<T> BinderInner<T>
{
    fn new(data: T) -> Self { Self { data, listeners: Vec::new(), next_id: 0 } }

    fn notify(&mut self)
    {
        for (_, meta) in self.listeners.iter_mut()
        {
            if meta.received_event
            {
                continue;
            }
            meta.received_event = true;
            (meta.listener)(&self.data);
            meta.received_event = false;
        }
    }
}

#[derive(Clone)]
pub struct Binder<T>(Arc<Mutex<BinderInner<T>>>)
where T: Copy;

impl<T> Binder<T>
where T: Copy + Send + 'static
{
    pub fn new(data: T) -> Self { Self(Arc::new(Mutex::new(BinderInner::new(data)))) }

    pub fn get(&self) -> T { self.0.lock().unwrap().data }

    pub fn set(&self, value: T)
    {
        let mut inner = self.0.lock().unwrap();
        inner.data = value;
        inner.notify();
    }

    pub fn raise(&self) { self.0.lock().unwrap().notify(); }

    /// Returns a listener ID — keep it to remove later
    pub fn add_listener<F>(&self, listener: F) -> usize
    where F: Fn(&T) + Send + 'static
    {
        let mut inner = self.0.lock().unwrap();
        let id = inner.next_id;
        inner.listeners.push((
            id,
            ListenerMetaData {
                listener:       Box::new(listener),
                received_event: false,
            },
        ));
        inner.next_id += 1;
        id
    }

    pub fn remove_listener(&self, id: usize)
    {
        let mut inner = self.0.lock().unwrap();
        inner.listeners.retain(|(k, _)| *k != id);
    }

    pub fn remove_all_listeners(&self) { self.0.lock().unwrap().listeners.clear(); }
}
