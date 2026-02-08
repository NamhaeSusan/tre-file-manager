//! Git branch and HEAD information.
//!
//! Retrieves the current branch name, short commit hash, detached-HEAD
//! state, and dirty-tree flag for a repository.

use std::path::Path;

use git2::Repository;

use crate::error::{CoreError, CoreResult};

/// Summary of the repository's current branch state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchInfo {
    /// Branch name, or `"HEAD"` when detached.
    pub name: String,
    /// `true` when HEAD does not point to a branch.
    pub is_detached: bool,
    /// First 7 characters of the HEAD commit hash, if available.
    pub commit_short: Option<String>,
    /// `true` when the working tree has any uncommitted changes.
    pub is_dirty: bool,
}

/// Returns branch information for the repository at `repo_root`.
///
/// Returns `Ok(None)` when the path is not a git repository or when the
/// repository has no commits yet (empty/unborn HEAD).
///
/// # Errors
///
/// Returns [`CoreError::Git`] if the repository exists but `git2` cannot
/// read HEAD (for reasons other than an unborn branch).
pub fn get_branch_info(repo_root: &Path) -> CoreResult<Option<BranchInfo>> {
    let repo = match Repository::open(repo_root) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    let head = match repo.head() {
        Ok(h) => h,
        // Unborn HEAD (no commits yet)
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => return Ok(None),
        Err(e) => return Err(CoreError::Git(e.message().to_string())),
    };

    let is_detached = repo.head_detached().unwrap_or(false);

    let name = if is_detached {
        "HEAD".to_string()
    } else {
        head.shorthand().unwrap_or("HEAD").to_string()
    };

    let commit_short = head.target().map(|oid| oid.to_string()[..7].to_string());

    let is_dirty = check_dirty(&repo);

    Ok(Some(BranchInfo {
        name,
        is_detached,
        commit_short,
        is_dirty,
    }))
}

/// Returns `true` if any file in the working tree or index has changes.
fn check_dirty(repo: &Repository) -> bool {
    let statuses = match repo.statuses(None) {
        Ok(s) => s,
        Err(_) => return false,
    };

    statuses.iter().any(|entry| {
        let s = entry.status();
        !s.is_ignored() && s != git2::Status::CURRENT
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper: create a git repo with an initial commit.
    fn setup_git_repo() -> (TempDir, git2::Repository) {
        let tmp = TempDir::new().unwrap();
        let repo = git2::Repository::init(tmp.path()).unwrap();

        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@test.com").unwrap();
        }

        {
            let sig = git2::Signature::now("Test User", "test@test.com").unwrap();
            let tree_id = {
                let mut index = repo.index().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
                .unwrap();
        }

        (tmp, repo)
    }

    /// Helper: stage and commit a file.
    fn commit_file(repo: &git2::Repository, path: &std::path::Path, content: &str) {
        fs::write(path, content).unwrap();
        let workdir = repo.workdir().unwrap().canonicalize().unwrap();
        let canon_path = path.canonicalize().unwrap();
        let relative = canon_path.strip_prefix(&workdir).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(relative).unwrap();
        index.write().unwrap();

        let sig = git2::Signature::now("Test User", "test@test.com").unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "add file", &tree, &[&head])
            .unwrap();
    }

    // --- get_branch_info with commits ---

    #[test]
    fn get_branch_info_with_commits_has_branch_name() {
        let (tmp, _repo) = setup_git_repo();

        let info = get_branch_info(tmp.path()).unwrap();
        assert!(info.is_some());
        let info = info.unwrap();
        // Default branch is typically "main" or "master"
        assert!(!info.name.is_empty());
        assert!(!info.is_detached);
    }

    #[test]
    fn get_branch_info_with_commits_has_commit_short() {
        let (tmp, _repo) = setup_git_repo();

        let info = get_branch_info(tmp.path()).unwrap().unwrap();
        assert!(info.commit_short.is_some());
        let short = info.commit_short.unwrap();
        assert_eq!(
            short.len(),
            7,
            "commit_short should be 7 chars, got: {short}"
        );
    }

    #[test]
    fn get_branch_info_clean_repo_not_dirty() {
        let (tmp, _repo) = setup_git_repo();

        let info = get_branch_info(tmp.path()).unwrap().unwrap();
        assert!(!info.is_dirty);
    }

    #[test]
    fn get_branch_info_dirty_repo() {
        let (tmp, _repo) = setup_git_repo();
        // Create an untracked file to make the repo dirty
        fs::write(tmp.path().join("dirty.txt"), "uncommitted").unwrap();

        let info = get_branch_info(tmp.path()).unwrap().unwrap();
        assert!(info.is_dirty);
    }

    #[test]
    fn get_branch_info_dirty_with_modified_file() {
        let (tmp, repo) = setup_git_repo();
        let file_path = tmp.path().join("file.txt");
        commit_file(&repo, &file_path, "original");
        fs::write(&file_path, "modified").unwrap();

        let info = get_branch_info(tmp.path()).unwrap().unwrap();
        assert!(info.is_dirty);
    }

    // --- empty repo ---

    #[test]
    fn get_branch_info_empty_repo_returns_none() {
        let tmp = TempDir::new().unwrap();
        // Init a repo but make NO commits (unborn HEAD)
        git2::Repository::init(tmp.path()).unwrap();

        let info = get_branch_info(tmp.path()).unwrap();
        assert!(
            info.is_none(),
            "empty repo with no commits should return None"
        );
    }

    // --- non-git directory ---

    #[test]
    fn get_branch_info_non_git_dir_returns_none() {
        let tmp = TempDir::new().unwrap();

        let info = get_branch_info(tmp.path()).unwrap();
        assert!(info.is_none());
    }

    // --- detached HEAD ---

    #[test]
    fn get_branch_info_detached_head() {
        let (tmp, repo) = setup_git_repo();

        // Get the HEAD commit OID and check out detached
        let head_oid = repo.head().unwrap().target().unwrap();
        repo.set_head_detached(head_oid).unwrap();

        let info = get_branch_info(tmp.path()).unwrap().unwrap();
        assert!(info.is_detached);
        assert_eq!(info.name, "HEAD");
    }

    // --- BranchInfo struct tests ---

    #[test]
    fn branch_info_clone_and_eq() {
        let info = BranchInfo {
            name: "main".to_string(),
            is_detached: false,
            commit_short: Some("abc1234".to_string()),
            is_dirty: false,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn branch_info_debug_format() {
        let info = BranchInfo {
            name: "develop".to_string(),
            is_detached: false,
            commit_short: Some("abc1234".to_string()),
            is_dirty: true,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("develop"));
        assert!(debug.contains("is_dirty: true"));
    }

    #[test]
    fn branch_info_ne_when_different() {
        let info1 = BranchInfo {
            name: "main".to_string(),
            is_detached: false,
            commit_short: Some("abc1234".to_string()),
            is_dirty: false,
        };
        let info2 = BranchInfo {
            name: "develop".to_string(),
            is_detached: false,
            commit_short: Some("abc1234".to_string()),
            is_dirty: false,
        };
        assert_ne!(info1, info2);
    }

    // --- check_dirty tests ---

    #[test]
    fn check_dirty_clean_repo() {
        let (tmp, repo) = setup_git_repo();
        let _ = tmp; // keep alive
        assert!(!check_dirty(&repo));
    }

    #[test]
    fn check_dirty_with_untracked_file() {
        let (tmp, repo) = setup_git_repo();
        fs::write(tmp.path().join("new.txt"), "content").unwrap();
        assert!(check_dirty(&repo));
    }

    #[test]
    fn check_dirty_with_modified_file() {
        let (tmp, repo) = setup_git_repo();
        let file_path = tmp.path().join("file.txt");
        commit_file(&repo, &file_path, "original");
        fs::write(&file_path, "changed").unwrap();
        assert!(check_dirty(&repo));
    }

    #[test]
    fn check_dirty_with_deleted_file() {
        let (tmp, repo) = setup_git_repo();
        let file_path = tmp.path().join("file.txt");
        commit_file(&repo, &file_path, "will be deleted");
        fs::remove_file(&file_path).unwrap();
        assert!(check_dirty(&repo));
    }

    // --- Multiple branches ---

    #[test]
    fn get_branch_info_on_new_branch() {
        let (tmp, repo) = setup_git_repo();

        // Create and checkout a new branch
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature-x", &head_commit, false).unwrap();
        repo.set_head("refs/heads/feature-x").unwrap();

        let info = get_branch_info(tmp.path()).unwrap().unwrap();
        assert_eq!(info.name, "feature-x");
        assert!(!info.is_detached);
    }
}
