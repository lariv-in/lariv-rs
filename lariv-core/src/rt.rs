//! Process / thread runtime helpers for deep HList install/mount chains.
//!
//! Prefer [`crate::main`] to annotate `async fn main` rather than calling these
//! directly.

/// Default stack size used by [`crate::main`] when `stack_size` is omitted (64 MiB).
pub const DEFAULT_STACK_SIZE: usize = 64 * 1024 * 1024;

/// Raise the process stack soft limit so `thread::Builder::stack_size` is not clipped by
/// `ulimit -s` (often 8192 KiB). Without this, requesting 32–64 MiB still leaves ~8 MiB.
#[cfg(unix)]
pub fn raise_process_stack_limit(bytes: usize) {
    use std::mem::MaybeUninit;

    let mut lim = MaybeUninit::<libc::rlimit>::uninit();
    // SAFETY: `lim` is a valid `rlimit`-sized buffer for `getrlimit`.
    let rc = unsafe { libc::getrlimit(libc::RLIMIT_STACK, lim.as_mut_ptr()) };
    if rc != 0 {
        return;
    }
    // SAFETY: `getrlimit` succeeded, so `lim` is initialized.
    let mut lim = unsafe { lim.assume_init() };
    let want = bytes as libc::rlim_t;
    lim.rlim_cur = if lim.rlim_max == libc::RLIM_INFINITY {
        want
    } else {
        want.min(lim.rlim_max)
    };
    // SAFETY: `lim` is a fully initialized `rlimit`.
    let rc = unsafe { libc::setrlimit(libc::RLIMIT_STACK, &lim) };
    if rc != 0 {
        eprintln!(
            "warning: could not raise stack limit to {} MiB; \
             try `ulimit -s unlimited` before starting the server",
            bytes / (1024 * 1024)
        );
    }
}

#[cfg(not(unix))]
pub fn raise_process_stack_limit(_bytes: usize) {}

/// Join a server thread and exit the process on error or panic.
pub fn join_server_thread<E: std::fmt::Display>(result: std::thread::Result<Result<(), E>>) {
    match result {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            eprintln!("{e:#}");
            std::process::exit(1);
        }
        Err(_) => {
            eprintln!("server thread panicked");
            std::process::exit(1);
        }
    }
}
