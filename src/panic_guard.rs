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
            (*callback_copy)();
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
    // Need this to communicate results back to us from within callback
    use std::sync::Mutex;

    // ensure that tests don't run in parallel while we're messing with the global panic hook
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_callback_called_on_drop() {
        let _m = TEST_MUTEX.lock().unwrap();

        let called = Arc::new(Mutex::new(0));

        {
            let called = called.clone();
            let _guard = GuardWithHook::new(move || {*called.lock().unwrap() += 1;});
            // guard is dropped, should call hook
        }

        assert_eq!(*called.lock().unwrap(), 1);
    }

    #[test]
    fn test_callback_called_once_only_panic() {
        let _m = TEST_MUTEX.lock().unwrap();
        let original_hook = std::panic::take_hook(); // so we can restore the original panic hook

        // test that the callback is only called once, even if there is a panic

        let calls = Arc::new(Mutex::new(0));

        assert!(std::panic::catch_unwind(|| {
            let calls = calls.clone();
            let _guard = GuardWithHook::new(move || {*calls.lock().unwrap() += 1;});
            panic!("test");
        }).is_err());

        // restore original hook before assert, so we see the error message if the test fails
        std::panic::set_hook(original_hook);

        assert_eq!(*calls.lock().unwrap(), 1);
    }

    #[test]
    fn test_callback_called_before_panic_hook() {
        let _m = TEST_MUTEX.lock().unwrap();
        let original_hook = std::panic::take_hook(); // so we can restore the original panic hook

        // test that the cleanup callback is called before the panic hook

        let calls: Arc<Mutex<Vec<&str>>> = Arc::new(Mutex::new(vec![]));
        let calls2 = calls.clone();
        std::panic::set_hook(Box::new(move |_| calls2.lock().unwrap().push("hook")));

        assert!(std::panic::catch_unwind(|| {
            let calls = calls.clone();
            let _guard = GuardWithHook::new(move || calls.lock().unwrap().push("cleanup"));
            panic!("test");
        }).is_err());

        // restore original hook before assert, so we see the error message if the test fails
        std::panic::set_hook(original_hook);

        assert_eq!(*calls.lock().unwrap(), vec!["cleanup", "hook"]);
    }

    #[test]
    fn test_nested_callback() {
        let _m = TEST_MUTEX.lock().unwrap();
        let original_hook = std::panic::take_hook(); // so we can restore the original panic hook

        let calls: Arc<Mutex<Vec<&str>>> = Arc::new(Mutex::new(vec![]));
        let calls2 = calls.clone();
        let calls3 = calls.clone();

        {
            let _g = GuardWithHook::new(move || calls2.lock().unwrap().push("outer"));
            {
                let _g = GuardWithHook::new(move || calls3.lock().unwrap().push("inner"));
            }
        }

        std::panic::set_hook(original_hook);

        assert_eq!(*calls.lock().unwrap(), vec!["inner", "outer"]);
    }

    #[test]
    fn test_nested_callback_with_panic() {
        let _m = TEST_MUTEX.lock().unwrap();
        let original_hook = std::panic::take_hook(); // so we can restore the original panic hook

        let calls: Arc<Mutex<Vec<&str>>> = Arc::new(Mutex::new(vec![]));
        let calls2 = calls.clone();
        let calls3 = calls.clone();

        assert!(std::panic::catch_unwind(|| {
            let _g = GuardWithHook::new(move || calls2.lock().unwrap().push("outer"));
            {
                let _g = GuardWithHook::new(move || calls3.lock().unwrap().push("inner"));
                panic!("test");
            }
        }).is_err());

        std::panic::set_hook(original_hook);
        assert_eq!(*calls.lock().unwrap(), vec!["inner", "outer"]);
    }

    #[test]
    fn test_nested_callback_hook_restored() {
        let _m = TEST_MUTEX.lock().unwrap();
        let original_hook = std::panic::take_hook(); // so we can restore the original panic hook

        let calls: Arc<Mutex<Vec<&str>>> = Arc::new(Mutex::new(vec![]));
        let calls2 = calls.clone();
        let calls3 = calls.clone();
        let calls4 = calls.clone();
        let calls5 = calls.clone();

        std::panic::set_hook(Box::new(move |_| calls2.lock().unwrap().push("outer hook")));

        {
            let _g = GuardWithHook::new(move || calls3.lock().unwrap().push("inner cleanup"));
            {
                let _g = GuardWithHook::new(move || calls4.lock().unwrap().push("inner inner cleanup"));

                // just for kicks, overwrite the panic hook here to check that it get's
                // overwritten when guard is dropped
                std::panic::set_hook(Box::new(move |_| calls5.lock().unwrap().push("wrong")));
            }

        }

        // if this assert fails, we will get no error message in stder...
        assert_eq!(*calls.lock().unwrap(), vec!["inner inner cleanup", "inner cleanup"]);

        // panic after both guards have gone out of scope, so only the outer hook should be called
        assert!(std::panic::catch_unwind(|| {
            panic!("test");
        }).is_err());

        // restore original hook before assert, so we see the error message if the test fails
        std::panic::set_hook(original_hook);
        assert_eq!(*calls.lock().unwrap(), vec!["inner inner cleanup", "inner cleanup", "outer hook"]);
    }

}