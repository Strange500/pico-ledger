#![no_std]

extern crate alloc;

#[cfg(feature = "psbt-demo")]
pub mod demo;
pub mod psbt;
pub mod signals;
pub mod ui;
pub mod wallet;

#[cfg(all(test, feature = "psbt-demo"))]
mod review_tests;
#[cfg(test)]
mod tests;
