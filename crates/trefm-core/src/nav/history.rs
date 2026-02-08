//! Navigation history with back/forward support.

use std::path::PathBuf;

/// Immutable navigation history with back/forward stacks.
///
/// Every mutation returns a **new** `History` instance, following the
/// project-wide immutability convention. Navigating forward after going
/// back is supported; pushing a new path clears the forward stack (same
/// semantics as a web browser).
#[derive(Debug, Clone, Default)]
pub struct History {
    back_stack: Vec<PathBuf>,
    forward_stack: Vec<PathBuf>,
}

impl History {
    /// Creates an empty history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pushes `path` onto the back stack and clears the forward stack.
    ///
    /// Returns a new `History`.
    pub fn push(&self, path: PathBuf) -> Self {
        let mut back_stack = self.back_stack.clone();
        back_stack.push(path);
        Self {
            back_stack,
            forward_stack: Vec::new(),
        }
    }

    /// Go back one step. Returns the new History and the path to navigate to,
    /// or `None` if the back stack is empty.
    pub fn go_back(&self) -> Option<(Self, PathBuf)> {
        if self.back_stack.is_empty() {
            return None;
        }
        let mut back_stack = self.back_stack.clone();
        let path = back_stack.pop()?;
        let mut forward_stack = self.forward_stack.clone();
        forward_stack.push(path.clone());
        let new_history = Self {
            back_stack,
            forward_stack,
        };
        Some((new_history, path))
    }

    /// Go forward one step. Returns the new History and the path to navigate to,
    /// or `None` if the forward stack is empty.
    pub fn go_forward(&self) -> Option<(Self, PathBuf)> {
        if self.forward_stack.is_empty() {
            return None;
        }
        let mut forward_stack = self.forward_stack.clone();
        let path = forward_stack.pop()?;
        let mut back_stack = self.back_stack.clone();
        back_stack.push(path.clone());
        let new_history = Self {
            back_stack,
            forward_stack,
        };
        Some((new_history, path))
    }

    /// Returns `true` if there is at least one entry on the back stack.
    pub fn can_go_back(&self) -> bool {
        !self.back_stack.is_empty()
    }

    /// Returns `true` if there is at least one entry on the forward stack.
    pub fn can_go_forward(&self) -> bool {
        !self.forward_stack.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn new_history_is_empty() {
        let history = History::new();
        assert!(!history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn push_enables_go_back() {
        let history = History::new();
        let history = history.push(PathBuf::from("/home"));

        assert!(history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn push_does_not_mutate_original() {
        let history = History::new();
        let _new_history = history.push(PathBuf::from("/home"));

        assert!(!history.can_go_back());
    }

    #[test]
    fn go_back_returns_pushed_path() {
        let history = History::new();
        let history = history.push(PathBuf::from("/home"));

        let (new_history, path) = history.go_back().unwrap();
        assert_eq!(path, PathBuf::from("/home"));
        assert!(!new_history.can_go_back());
        assert!(new_history.can_go_forward());
    }

    #[test]
    fn go_back_on_empty_returns_none() {
        let history = History::new();
        assert!(history.go_back().is_none());
    }

    #[test]
    fn go_forward_after_go_back() {
        let history = History::new();
        let history = history.push(PathBuf::from("/home"));

        let (history, _) = history.go_back().unwrap();
        let (history, path) = history.go_forward().unwrap();

        assert_eq!(path, PathBuf::from("/home"));
        assert!(history.can_go_back());
        assert!(!history.can_go_forward());
    }

    #[test]
    fn go_forward_on_empty_returns_none() {
        let history = History::new();
        assert!(history.go_forward().is_none());
    }

    #[test]
    fn push_clears_forward_stack() {
        let history = History::new();
        let history = history.push(PathBuf::from("/home"));
        let history = history.push(PathBuf::from("/projects"));

        let (history, _) = history.go_back().unwrap();
        assert!(history.can_go_forward());

        let history = history.push(PathBuf::from("/documents"));
        assert!(!history.can_go_forward());
        assert!(history.can_go_back());
    }

    #[test]
    fn multiple_push_and_back() {
        let history = History::new();
        let history = history.push(PathBuf::from("/a"));
        let history = history.push(PathBuf::from("/b"));
        let history = history.push(PathBuf::from("/c"));

        let (history, path) = history.go_back().unwrap();
        assert_eq!(path, PathBuf::from("/c"));

        let (history, path) = history.go_back().unwrap();
        assert_eq!(path, PathBuf::from("/b"));

        let (history, path) = history.go_back().unwrap();
        assert_eq!(path, PathBuf::from("/a"));

        assert!(history.go_back().is_none());
    }

    #[test]
    fn back_and_forward_round_trip() {
        let history = History::new();
        let history = history.push(PathBuf::from("/a"));
        let history = history.push(PathBuf::from("/b"));

        let (history, path_b) = history.go_back().unwrap();
        assert_eq!(path_b, PathBuf::from("/b"));

        let (history, path_a) = history.go_back().unwrap();
        assert_eq!(path_a, PathBuf::from("/a"));

        let (history, fwd_a) = history.go_forward().unwrap();
        assert_eq!(fwd_a, PathBuf::from("/a"));

        let (history, fwd_b) = history.go_forward().unwrap();
        assert_eq!(fwd_b, PathBuf::from("/b"));

        assert!(!history.can_go_forward());
    }

    #[test]
    fn default_is_same_as_new() {
        let h1 = History::new();
        let h2 = History::default();

        assert!(!h1.can_go_back());
        assert!(!h1.can_go_forward());
        assert!(!h2.can_go_back());
        assert!(!h2.can_go_forward());
    }

    #[test]
    fn clone_produces_independent_copy() {
        let history = History::new();
        let history = history.push(PathBuf::from("/home"));

        let cloned = history.clone();
        assert!(cloned.can_go_back());

        let (cloned_back, _) = cloned.go_back().unwrap();
        assert!(!cloned_back.can_go_back());
        assert!(history.can_go_back());
    }
}
