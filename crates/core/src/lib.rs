#![no_std]

extern crate alloc;

#[cfg(feature = "psbt-demo")]
pub mod demo;
pub mod psbt;
pub mod signals;
pub mod ui;
pub mod wallet;

#[cfg(test)]
mod tests;
