use std::sync::{Arc, Condvar, Mutex};

struct FutureData<T> {
    value: Mutex<Option<T>>,
    ready: Condvar,
}

#[derive(Clone)]
pub struct Future<T: Clone> {
    v: Arc<FutureData<T>>,
}

impl<T: Clone> Future<T> {
    pub fn new() -> Future<T> {
        return Future {
            v: Arc::new(FutureData {
                value: Mutex::new(None),
                ready: Condvar::new(),
            }),
        };
    }
    pub fn wait(&mut self) -> T {
        loop {
            let guard = (*self.v).value.lock().unwrap();
            if let Some(ref value) = *guard {
                return value.clone();
            }
            let _new_guard = (*self.v).ready.wait(guard).unwrap();
        }
    }
    pub fn set(&mut self, value: T) {
        (*(*self.v).value.lock().unwrap()) = Some(value);
        (*self.v).ready.notify_all();
    }
}
