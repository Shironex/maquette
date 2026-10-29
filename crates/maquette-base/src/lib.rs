//! The bottom layer every other Maquette crate builds on: the error type
//! every fallible function returns, file and JSON helpers, and the lookup of
//! the external tools Maquette drives. It uses no other Maquette crate.

pub mod error;
pub mod fs;
pub mod json;
pub mod toolchain;
