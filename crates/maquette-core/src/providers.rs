//! Sending to the 3D services. The work lives in the providers crate; this
//! keeps the old path, and its tests drive a send through to the inbox,
//! which sits above that crate.

pub use maquette_providers::*;

#[cfg(test)]
mod tests;
