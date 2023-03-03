# rendezvous

Rust implementation of Ada's synchronous inter-thread communication primitive (rendezvous)

The code in its current state is incorrect. `(*self.tx).thread.unpark()` in `ScopedSender::send()` is racy as it is called after setting `is_set` to true.
