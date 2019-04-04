/// `assert!` that is exists only if `"no-slow-safety-checks"` feature is not enabled.
#[cfg(not(feature = "no-slow-safety-checks"))]
#[macro_export]
macro_rules! rendy_slow_assert {
    ($($arg:tt)*) => {
        assert!($($arg)*);
    }
}

/// `assert_eq!` that is exists only if `"no-slow-safety-checks"` feature is not enabled.
#[cfg(not(feature = "no-slow-safety-checks"))]
#[macro_export]
macro_rules! rendy_slow_assert_eq {
    ($($arg:tt)*) => {
        assert_eq!($($arg)*);
    }
}

/// `assert_ne!` that is exists only if `"no-slow-safety-checks"` feature is not enabled.
#[cfg(not(feature = "no-slow-safety-checks"))]
#[macro_export]
macro_rules! rendy_slow_assert_ne {
    ($($arg:tt)*) => {
        assert_ne!($($arg)*);
    }
}

/// `assert!` that is exists only if `"no-slow-safety-checks"` feature is not enabled.
#[cfg(feature = "no-slow-safety-checks")]
#[macro_export]
macro_rules! rendy_slow_assert {
    ($($arg:tt)*) => {
        assert!($($arg)*);
    }
}

/// `assert_eq!` that is exists only if `"no-slow-safety-checks"` feature is not enabled.
#[cfg(feature = "no-slow-safety-checks")]
#[macro_export]
macro_rules! rendy_slow_assert_eq {
    ($($arg:tt)*) => {
        assert_eq!($($arg)*);
    }
}

/// `assert_ne!` that is exists only if `"no-slow-safety-checks"` feature is not enabled.
#[cfg(feature = "no-slow-safety-checks")]
#[macro_export]
macro_rules! rendy_slow_assert_ne {
    ($($arg:tt)*) => {
        assert_ne!($($arg)*);
    }
}
