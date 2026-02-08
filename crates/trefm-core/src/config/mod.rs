//! Configuration management for TreFM.
//!
//! User preferences ([`settings::Config`]) and key bindings ([`keymap::Keymap`])
//! are stored as TOML files and loaded at startup.

pub mod keymap;
pub mod settings;
pub mod theme;
