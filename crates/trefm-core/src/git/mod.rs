//! Git integration for TreFM.
//!
//! Provides file-level git status ([`status`]) and branch information
//! ([`branch`]) by wrapping `git2`.

pub mod branch;
pub mod status;

pub use branch::BranchInfo;
pub use status::GitFileStatus;
