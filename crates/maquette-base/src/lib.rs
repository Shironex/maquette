//! The bottom layer every other Maquette crate builds on: the error type
//! every fallible function returns, file and JSON helpers, the clock files
//! write timestamps with, and the lookup of the external tools Maquette
//! drives. It uses no other Maquette crate.

pub mod clock;
pub mod error;
pub mod fs;
pub mod json;
pub mod toolchain;
