//! `Instant` alias per target. `std::time::Instant::now` panics on
//! wasm32-unknown-unknown ("time not implemented on this platform"), so the
//! browser build sources it from web-time (`performance.now()`).
#[cfg(not(target_family = "wasm"))]
pub(crate) use std::time::Instant;
#[cfg(target_family = "wasm")]
pub(crate) use web_time::Instant;
