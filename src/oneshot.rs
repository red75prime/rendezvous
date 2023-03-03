use std::marker::PhantomData;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::Thread;

struct OneShot<T> {
    thread: Thread,
    is_set: AtomicBool,
    value: UnsafeCell<Option<T>>,
}

impl<T> OneShot<T> {
    fn wait_for_is_set(&self) {
        while !self.is_set.load(Ordering::SeqCst) {
            std::thread::park();
        }
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
    sent: bool,
}

unsafe impl<T: Send> Send for ScopedSender<T> {}

impl<T> ScopedSender<T> {
    pub fn send(mut self, val: T)
    where
        T: Send,
    {
        unsafe {
            // OneShotReceiver doesn't read value until is_set is true
            *(*self.tx).value.get() = Some(val);
            (*self.tx).is_set.store(true, Ordering::SeqCst);
            (*self.tx).thread.unpark();
        }
        self.sent = true;
    }
}

impl<T> Drop for ScopedSender<T> {
    fn drop(&mut self) {
        unsafe {
            // If send() was called, the tx pointer might be invalid
            if !self.sent {
                (*self.tx).is_set.store(true, Ordering::SeqCst);
                (*self.tx).thread.unpark();
            }
        }
    }
}

pub struct ScopedReceiver<'scope, T> {
    rx: &'scope OneShot<T>,
    // Not send
    _ph: PhantomData<*mut T>,
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
        oneshot: OneShot { thread: std::thread::current(),
            is_set: AtomicBool::new(false),
            value: UnsafeCell::new(None),
        },
        scope: PhantomData,
    };
    let scoped_sender = ScopedSender {
        tx: &scope.oneshot,
        sent: false,
    };
    let scoped_receiver = ScopedReceiver {
        rx: &scope.oneshot,
        _ph: PhantomData,
    };
    let r = f(scoped_sender, scoped_receiver);
    // make sure that scope.oneshot is alive until the value is actually set
    scope.oneshot.wait_for_is_set();
    r
}
