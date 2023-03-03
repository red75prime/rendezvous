use std::marker::PhantomData;
use std::cell::UnsafeCell;
use std::sync::{Mutex, Condvar};

struct OneShot<T> {
    cvar: Condvar,
    is_set: Mutex<bool>,
    value: UnsafeCell<Option<T>>,
}

impl<T> OneShot<T> {
    fn wait_for_is_set(&self) {
        let guard = self.is_set.lock().unwrap();
        // We don't need the returned mutex guard
        drop(self.cvar.wait_while(guard, |is_set| !*is_set ).unwrap());
    }
}

unsafe impl<T: Send> Send for OneShot<T> {}
unsafe impl<T: Send> Sync for OneShot<T> {}

pub struct Scope<'scope, T> {
    oneshot: OneShot<T>,
    scope: PhantomData<&'scope mut &'scope ()>,
}

pub struct ScopedSender<T> {
    // We ensure validity of the pointer by making sure that Scope that contains OneShot
    // lives until Sender signals that the value is set
    tx: *const OneShot<T>,
}

unsafe impl<T: Send> Send for ScopedSender<T> {}

impl<T> ScopedSender<T> {
    pub fn send(self, val: T)
    where
        T: Send,
    {
        unsafe {
            // OneShotReceiver doesn't read value until is_set is true
            *(*self.tx).value.get() = Some(val);
            let mut is_set_guard = (*self.tx).is_set.lock().unwrap();
            *is_set_guard = true;
            (*self.tx).cvar.notify_one();
        }
    }
}

pub struct ScopedReceiver<'scope, T> {
    rx: &'scope OneShot<T>,
    // Not send
    _ph: PhantomData<*mut T>,
}

impl<'scope, T> Drop for ScopedReceiver<'scope, T> {
    fn drop(&mut self) {
        // make sure that `OneShot` is alive until `is_set` is true
        self.rx.wait_for_is_set();
    }
}

impl<'scope, T> ScopedReceiver<'scope, T> {
    pub fn recv(self) -> T
    where
        T: Send,
    {
        self.rx.wait_for_is_set();
        unsafe {
            (*self.rx.value.get()).take().expect("Sending thread panicked or exited")
        }
    }
}

pub fn scoped_oneshot_channel<T, F, R>(f: F) -> R
where
    F: for<'scope> FnOnce(ScopedSender<T>, ScopedReceiver<'scope, T>) -> R,
    R: Send,
{
    let scope = Scope {
        oneshot: OneShot { 
            cvar: Condvar::new(),
            is_set: Mutex::new(false),
            value: UnsafeCell::new(None),
        },
        scope: PhantomData,
    };
    let scoped_sender = ScopedSender {
        tx: &scope.oneshot,
    };
    let scoped_receiver = ScopedReceiver {
        rx: &scope.oneshot,
        _ph: PhantomData,
    };
    f(scoped_sender, scoped_receiver)
}
