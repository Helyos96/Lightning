// todo: remove this once stable-ish
#![allow(dead_code, unused_imports)]

#[macro_use]
mod macros;
pub mod build;
pub mod calc;
pub mod data;
pub mod gem;
mod gemstats;
pub mod item;
mod modifier;
pub mod tree;
pub mod util;

#[cfg(feature="import")]
pub mod import;