// todo: remove this once stable-ish
#![allow(dead_code, unused_imports)]

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[macro_use]
pub mod macros;
pub mod build;
pub mod calc;
pub mod data;
pub mod gem;
mod gemstats;
pub mod item;
pub mod modifier;
pub mod tree;
pub mod util;
pub mod stackvec;

#[cfg(feature="import")]
pub mod import;