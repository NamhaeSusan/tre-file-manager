//! Navigation logic for TreFM.
//!
//! This module contains the [`panel::Panel`] trait, the [`panel::SinglePanel`]
//! implementation, navigation [`history::History`], [`bookmarks::Bookmarks`],
//! and entry [`filter`]ing/sorting (including fuzzy search).

pub mod bookmarks;
pub mod filter;
pub mod history;
pub mod panel;
