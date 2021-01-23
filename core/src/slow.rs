//! Macros that do run-time safety checks. These can be disabled, but this increases
//! the risk of unsafe behavior.

/// `assert!` that is exists only if `"no-slow-safety-checks"` feature is not enabled.
#[macro_export]
macro_rules! rendy_slow_assert {
    ($($tt:tt)*) => {};
}

/// `assert_eq!` that is exists only if `"no-slow-safety-checks"` feature is not enabled.
#[macro_export]
macro_rules! rendy_slow_assert_eq {
    ($($tt:tt)*) => {};
}

/// `assert_ne!` that is exists only if `"no-slow-safety-checks"` feature is not enabled.
#[macro_export]
macro_rules! rendy_slow_assert_ne {
    ($($tt:tt)*) => {};
}
