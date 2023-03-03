#[cfg(target_env = "uclibc")]
mod pthread;

mod oneshot;

macro_rules! task {
    {$Task:ident, $Methods:ident, $Selector:ident {
        $(
            fn ($method:ident: $G:ident) $Method:ident($( $arg:ident : $type:ty ),*) -> $ret:ty;
        )+
    } } => {
        enum $Methods<'scope> {
            $( $Method($( $type ,)* crate::oneshot::ScopedSender<$ret>) ,)*
            #[allow(dead_code)]
            Never(&'scope ()),
        }

        pub struct $Selector {
            rx: std::sync::mpsc::Receiver<$Methods<'static>>,
        }

        impl $Selector {
            #[allow(dead_code)]
            pub fn select_blocking<$( $G, )+>(&self, $( $method: $G ,)+) -> Result<(), ()>
            where
            $( $G: for<'scope> FnOnce($( $type ,)*) -> $ret  ),+
            {
                match self.rx.recv() {
                    $(
                        Ok($Methods::$Method($( $arg ,)* tx)) => {
                            tx.send($method($( $arg ,)*));
                            std::thread::yield_now();
                            Ok(())
                        }
                    )+
                    Ok($Methods::Never(_)) => Ok(()),
                    Err(_) => Err(()),
                }
            }

            #[allow(dead_code)]
            pub fn select_timeout<$( $G, )+>(&self, timeout: std::time::Duration, $( $method: $G ,)+) -> Result<(), std::sync::mpsc::RecvTimeoutError>
            where
            $( $G: for<'scope> FnOnce($( $type ,)*) -> $ret  ),+
            {
                match self.rx.recv_timeout(timeout)? {
                    $(
                        $Methods::$Method($( $arg ,)* tx) => {
                            tx.send($method($( $arg ,)*));
                            std::thread::yield_now();
                            Ok(())
                        }
                    )+
                    $Methods::Never(_) => Ok(()),
                }
            }
        }

        #[derive(Clone)]
        pub struct $Task {
            tx: std::sync::mpsc::SyncSender<$Methods<'static>>,
        }


        impl $Task {
            $(
                pub fn $method<'scope>(&self, $( $arg: $type  ),*) -> $ret {
                    crate::oneshot::scoped_oneshot_channel(|tx, rx| {
                        let ctx: &std::sync::mpsc::SyncSender<$Methods<'_>> = unsafe { core::mem::transmute(&self.tx) };
                        ctx.send($Methods::$Method( $( $arg ,)* tx)).expect(concat!(stringify!($Method)," failed"));
                        std::thread::yield_now();
                        rx.recv()
                    })
                }
            )+
            pub fn start_task<F>(f: F) -> $Task
            where
                F: 'static + Send + FnOnce($Selector),
            {
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                let selector = $Selector{ rx };
                std::thread::spawn(move || f(selector));
                $Task{ tx }
            }
        }
    };
}


mod test_task {
    task! {
        TestTask, TestMethods, TestSelector {
            fn (noop: F1) Noop() -> ();
            fn (test: F2) Test(s: &'scope str) -> String;
        }
    }
}

fn test_task(selector: test_task::TestSelector) {
    loop {
        let res = selector.select_blocking(
            || {
            },
            |s| {
                s.to_string() + " It's processed"
            });
        if res.is_err() {
            break;
        }
    }
}

fn main() {
    let test_task = test_task::TestTask::start_task(test_task);
    let cnt = 100_000;

    let start = std::time::Instant::now();
    for _ in 0..cnt {
        test_task.noop();
    }
    println!("one test_task.noop() call in {:?}", start.elapsed()/cnt);

    let mut r = String::new();
    let start = std::time::Instant::now();
    for _ in 0..cnt {
        r = test_task.test("aha");
        assert_eq!(r, "aha It's processed");
    }
    println!("one test_task.test() call in {:?}", start.elapsed()/cnt);
    println!("{r}");
}
