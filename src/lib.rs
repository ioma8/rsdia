//! rsdia — terminal ASCII diagram editor.
//!
//! `core` is pure: no terminal, no I/O. `storage` owns the XDG files. `tui` is the
//! ratatui front end, and `cli` is the argument surface.

pub mod cli;
pub mod core;
pub mod storage;
pub mod tui;
