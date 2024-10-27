macro_rules! regex {
    ($re:expr) => {
        Regex::new(&$re).unwrap()
    };
}

// Taken from maplit 1.0.2, renamed to hset
// License for this macro: MIT or Apache-2.0
#[macro_export]
macro_rules! hset {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(hset!(@single $rest)),*]));

    ($($key:expr,)+) => { hset!($($key),+) };
    ($($key:expr),*) => {
        {
            let mut _set = ::rustc_hash::FxHashSet::default();
            $(
                let _ = _set.insert($key);
            )*
            _set
        }
    };
}
