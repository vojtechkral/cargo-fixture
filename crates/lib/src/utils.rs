macro_rules! maybe_await {
    ( $($tt:tt)* ) => {{
        #[cfg(any(feature = "smol", feature = "tokio"))] { $($tt)*.await }
        #[cfg(not(any(feature = "smol", feature = "tokio") ))] { $($tt)* }
    }};
}

pub(crate) use cargo_fixture_macros::maybe_async;
pub(crate) use maybe_await;
