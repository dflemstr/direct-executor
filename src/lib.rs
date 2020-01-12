//! # `direct-executor`
//!
//! An executor that directly executes futures, with an optional customizable wait operation.
#![no_std]
#![deny(
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_import_braces,
    unused_qualifications,
    clippy::all
)]

use core::future;
use core::ptr;
use core::task;

const NOOP_WAKER: task::RawWaker = task::RawWaker::new(
    ptr::null(),
    &task::RawWakerVTable::new(|_| NOOP_WAKER, |_| {}, |_| {}, |_| {}),
);

/// Runs the provided future by spin-looping until polling succeeds.
///
/// This is equivalent to `run(future, || core::sync::atomic::spin_loop_hint())`.
pub fn run_spinning<F>(future: F) -> F::Output
where
    F: future::Future,
{
    run(future, || core::sync::atomic::spin_loop_hint())
}

/// Runs the provided future until polling succeeds, calling the provided `wait` closure in between
/// each polling attempt.
///
/// A common pattern is to let `wait` simply be some delay function (like `sleep()`), or in certain
/// environments (such as on embedded devices), it might make sense to call `wfi` to wait for
/// peripheral interrupts, if you know that to be the source of future completion.
pub fn run<F>(future: F, mut wait: impl FnMut()) -> F::Output
where
    F: future::Future,
{
    pin_utils::pin_mut!(future);
    let waker = unsafe { task::Waker::from_raw(NOOP_WAKER) };

    let mut context = task::Context::from_waker(&waker);
    loop {
        if let task::Poll::Ready(result) = future.as_mut().poll(&mut context) {
            return result;
        }
        wait();
    }
}
