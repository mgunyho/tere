use std::sync::Arc;

type PanicHookType = (dyn for<'r, 's> Fn(&'r std::panic::PanicInfo<'s>) + Send + Sync + 'static);

/// Custom scopeguard-like struct that wraps a panic hook function and a callback ("cleanup")
/// function, and in the case of a panic, calls the callback *before* the wrapped panic hook (i.e.
/// printing the error message to stderr). We need this to e.g. switch back to the non-alternate
/// screen before printing the error message, so that it doesn't disappear into the alternate
/// screen.
pub struct GuardWithHook<F>
where
    F: Fn() + Sync + Send + 'static,
{
    original_hook: Arc<PanicHookType>,
    callback: Arc<F>,
}

impl<F> GuardWithHook<F>
where
    F: Fn() + Sync + Send + 'static,
{
    /// Store a callback function and the current panic hook, and install a new panic hook that
    /// first calls the callback, and then the original.
    fn new(callback: F) -> Self {
        let callback = Arc::new(callback);
        let callback_copy = callback.clone();

        let original_hook: Arc<PanicHookType> = Arc::from(std::panic::take_hook());
        let original_hook_copy = original_hook.clone();

        std::panic::set_hook(Box::new(move |info| {
            //(*callback_copy)();
            (*original_hook_copy)(info);
        }));

        Self {
            original_hook,
            callback,
        }
    }
}

impl<F> Drop for GuardWithHook<F>
where
    F: Fn() + Sync + Send + 'static,
{
    /// Restore the original panic hook, and call the callback.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            // Set the panic hook back to what it was before. Note that this can be done only if
            // we're not panicking.
            let original_hook = self.original_hook.clone();
            std::panic::set_hook(Box::new(move |info| (*original_hook)(info)));

            // Only call the callback if we're not panicking, otherwise it has already been called
            // by the panic hook.
            (self.callback)();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Need these to communicate results back to us from within callback
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
    use std::sync::Mutex;

    #[test]
    fn test_callback_called_on_drop() {
        let called = Arc::new(Mutex::new(0));

        {
            let called = called.clone();
            let _guard = GuardWithHook::new(move || {*called.lock().unwrap() += 1;});
            // guard is dropped, should call hook
        }

        assert_eq!(*called.lock().unwrap(), 1);
    }

    #[test]
    fn test_panic() {
        // because we're messing with the global panic hook, all tests have to be in one function
        // to ensure that we're not modifying it from multiple different threads (unless we want to
        // run the tests single-threaded).

        // test that the callback is only called once, even if there is a panic
        //let calls = Arc::new(Mutex::new(0));
        //{
        //    let calls = calls.clone();
        //    let _guard = GuardWithHook::new(move || {*calls.lock().unwrap() += 1;});

        //    assert!(std::panic::catch_unwind(|| panic!("test")).is_err());
        //}
        //assert_eq!(*calls.lock().unwrap(), 1);

        // test that the callback is called before the panic hook

        let original_hook = std::panic::take_hook(); // this should be the default hook

        let calls: Arc<Mutex<Vec<&str>>> = Arc::new(Mutex::new(vec![]));
        let calls2 = calls.clone();
        std::panic::set_hook(Box::new(move |info| {
            if let Ok(mut v) = calls2.try_lock() {
                v.push("hook");
            }
            eprintln!("{}", info);
        }));
        {
            let calls = calls.clone();
            let _guard = GuardWithHook::new(move || {calls.lock().unwrap().push("cleanup");});

            assert!(std::panic::catch_unwind(|| panic!("test")).is_err());
        }
        assert_eq!(*calls.lock().unwrap(), vec!["cleanup", "hook"]);

        std::panic::set_hook(original_hook);
    }

}
