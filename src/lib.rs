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
use core::task;

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
pub fn run<F>(future: F, wait: impl FnMut()) -> F::Output
where
    F: future::Future,
{
    run_with_wake(future, wait, || {})
}

/// Runs the provided future until polling succeeds, calling the provided `wait` closure in between
/// each polling attempt.
///
/// When this thread is supposed to wake up again, the provided `wake` closure will be called.  This
/// allows the user to provide custom "unpark" functionality, if necessary.
///
/// A common pattern is to let `wait` simply be some delay function (like `sleep()`), or in certain
/// environments (such as on embedded devices), it might make sense to call `wfi` to wait for
/// peripheral interrupts, if you know that to be the source of future completion.
pub fn run_with_wake<F>(future: F, mut wait: impl FnMut(), wake: impl Fn()) -> F::Output
where
    F: future::Future,
{
    pin_utils::pin_mut!(future);
    let raw_waker: task::RawWaker = create_raw_waker(&wake);
    let waker = unsafe { task::Waker::from_raw(raw_waker) };

    let mut context = task::Context::from_waker(&waker);
    loop {
        if let task::Poll::Ready(result) = future.as_mut().poll(&mut context) {
            return result;
        }
        wait();
    }
}

fn create_raw_waker<F>(wake: *const F) -> task::RawWaker
where
    F: Fn(),
{
    task::RawWaker::new(
        wake as *const (),
        &task::RawWakerVTable::new(
            |wake_ptr| create_raw_waker(wake_ptr as *const F),
            |wake_ptr| unsafe {
                let wake = (wake_ptr as *const F).as_ref().unwrap();
                wake();
            },
            |wake_ptr| unsafe {
                let wake = (wake_ptr as *const F).as_ref().unwrap();
                wake();
            },
            |_| {},
        ),
    )
}
