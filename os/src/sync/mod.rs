//! Synchronization and interior mutability primitives
mod interrupt;
mod up;
pub use interrupt::{interrupt_get, interrupt_off, interrupt_on};
pub use up::*;
