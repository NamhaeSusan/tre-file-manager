//! Git file status detection.
//!
//! Maps each file in a repository to its [`GitFileStatus`] by inspecting the
//! working tree and index via `git2`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use git2::Repository;

use crate::error::{CoreError, CoreResult};

/// The git status of an individual file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFileStatus {
    /// File has been modified in the working tree or index.
    Modified,
    /// File is staged for addition (new file in index).
    Added,
    /// File has been deleted.
    Deleted,
    /// File has been renamed.
    Renamed,
    /// File is not tracked by git.
    Untracked,
    /// File is ignored via `.gitignore`.
    Ignored,
    /// File is unchanged (clean).
    Unchanged,
}

/// Returns `true` if the given path is inside a git repository.
pub fn is_git_repo(path: &Path) -> bool {
    find_repo_root(path).is_some()
}

/// Walks up from `path` to find the repository root.
///
/// Returns `None` if the path is not inside a git repository.
pub fn find_repo_root(path: &Path) -> Option<PathBuf> {
    Repository::discover(path)
        .ok()
        .and_then(|repo| repo.workdir().map(|w| w.to_path_buf()))
}

/// Returns the git status of every file in the repository rooted at `repo_root`.
///
/// Keys are **absolute paths**. Files that are clean (unchanged) are not
/// included — use [`get_status_for_path`] which defaults to
/// [`GitFileStatus::Unchanged`] for missing keys.
///
/// # Errors
///
/// Returns [`CoreError::Git`] if `repo_root` is not a valid git repository
/// or if `git2` fails to read the status list.
pub fn get_file_statuses(repo_root: &Path) -> CoreResult<HashMap<PathBuf, GitFileStatus>> {
    let repo = Repository::open(repo_root).map_err(|e| CoreError::Git(e.message().to_string()))?;

    let statuses = repo
        .statuses(None)
        .map_err(|e| CoreError::Git(e.message().to_string()))?;

    let mut map = HashMap::new();

    for entry in statuses.iter() {
        let Some(rel_path) = entry.path() else {
            continue;
        };
        let abs_path = repo_root.join(rel_path);
        let status = map_git2_status(entry.status());
        map.insert(abs_path, status);
    }

    Ok(map)
}

/// Looks up the status of a single path in a pre-computed status map.
///
/// Returns [`GitFileStatus::Unchanged`] when the path is not present in the
/// map (i.e. git considers it clean).
pub fn get_status_for_path(
    statuses: &HashMap<PathBuf, GitFileStatus>,
    path: &Path,
) -> GitFileStatus {
    statuses
        .get(path)
        .copied()
        .unwrap_or(GitFileStatus::Unchanged)
}

/// Maps `git2::Status` bit-flags to our simplified [`GitFileStatus`].
fn map_git2_status(status: git2::Status) -> GitFileStatus {
    if status.is_ignored() {
        return GitFileStatus::Ignored;
    }
    if status.is_index_new() {
        return GitFileStatus::Added;
    }
    if status.is_wt_new() {
        return GitFileStatus::Untracked;
    }
    if status.is_wt_renamed() || status.is_index_renamed() {
        return GitFileStatus::Renamed;
    }
    if status.is_wt_deleted() || status.is_index_deleted() {
        return GitFileStatus::Deleted;
    }
    if status.is_wt_modified()
        || status.is_index_modified()
        || status.is_wt_typechange()
        || status.is_index_typechange()
    {
        return GitFileStatus::Modified;
    }
    GitFileStatus::Unchanged
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

        // Configure user for commits
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test User").unwrap();
            config.set_str("user.email", "test@test.com").unwrap();
        }

        // Create initial commit with an empty tree
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
    fn commit_file(repo: &git2::Repository, path: &Path, content: &str) {
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

    // --- is_git_repo tests ---

    #[test]
    fn is_git_repo_inside_repo_returns_true() {
        let (tmp, _repo) = setup_git_repo();
        assert!(is_git_repo(tmp.path()));
    }

    #[test]
    fn is_git_repo_outside_repo_returns_false() {
        let tmp = TempDir::new().unwrap();
        assert!(!is_git_repo(tmp.path()));
    }

    #[test]
    fn is_git_repo_subdirectory_returns_true() {
        let (tmp, _repo) = setup_git_repo();
        let subdir = tmp.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        assert!(is_git_repo(&subdir));
    }

    // --- find_repo_root tests ---

    #[test]
    fn find_repo_root_finds_root() {
        let (tmp, _repo) = setup_git_repo();
        let root = find_repo_root(tmp.path());
        assert!(root.is_some());
        let expected = tmp.path().canonicalize().unwrap();
        let actual = root.unwrap().canonicalize().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn find_repo_root_from_subdirectory() {
        let (tmp, _repo) = setup_git_repo();
        let subdir = tmp.path().join("deep").join("nested");
        fs::create_dir_all(&subdir).unwrap();

        let root = find_repo_root(&subdir);
        assert!(root.is_some());
        let expected = tmp.path().canonicalize().unwrap();
        let actual = root.unwrap().canonicalize().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn find_repo_root_outside_repo_returns_none() {
        let tmp = TempDir::new().unwrap();
        assert!(find_repo_root(tmp.path()).is_none());
    }

    // --- get_file_statuses tests ---

    #[test]
    fn get_file_statuses_detects_untracked_file() {
        let (tmp, _repo) = setup_git_repo();
        fs::write(tmp.path().join("new_file.txt"), "hello").unwrap();

        let statuses = get_file_statuses(tmp.path()).unwrap();
        let abs_path = tmp.path().join("new_file.txt");
        assert_eq!(statuses.get(&abs_path), Some(&GitFileStatus::Untracked));
    }

    #[test]
    fn get_file_statuses_detects_modified_file() {
        let (tmp, repo) = setup_git_repo();
        let file_path = tmp.path().join("tracked.txt");
        commit_file(&repo, &file_path, "original");
        fs::write(&file_path, "modified content").unwrap();

        let statuses = get_file_statuses(tmp.path()).unwrap();
        assert_eq!(statuses.get(&file_path), Some(&GitFileStatus::Modified));
    }

    #[test]
    fn get_file_statuses_clean_file_not_in_map() {
        let (tmp, repo) = setup_git_repo();
        let file_path = tmp.path().join("clean.txt");
        commit_file(&repo, &file_path, "content");

        let statuses = get_file_statuses(tmp.path()).unwrap();
        // Clean files may be absent or Unchanged
        let status = statuses.get(&file_path);
        assert!(
            status.is_none() || status == Some(&GitFileStatus::Unchanged),
            "clean file should not appear or be Unchanged, got: {:?}",
            status
        );
    }

    #[test]
    fn get_file_statuses_detects_deleted_file() {
        let (tmp, repo) = setup_git_repo();
        let file_path = tmp.path().join("to_delete.txt");
        commit_file(&repo, &file_path, "will be deleted");
        fs::remove_file(&file_path).unwrap();

        let statuses = get_file_statuses(tmp.path()).unwrap();
        assert_eq!(statuses.get(&file_path), Some(&GitFileStatus::Deleted));
    }

    #[test]
    fn get_file_statuses_empty_repo_returns_empty() {
        let (tmp, _repo) = setup_git_repo();

        let statuses = get_file_statuses(tmp.path()).unwrap();
        assert!(statuses.is_empty());
    }

    #[test]
    fn get_file_statuses_multiple_files() {
        let (tmp, repo) = setup_git_repo();

        // One tracked-and-modified, one untracked, one clean
        let tracked = tmp.path().join("tracked.txt");
        commit_file(&repo, &tracked, "original");
        fs::write(&tracked, "modified").unwrap();

        let untracked = tmp.path().join("untracked.txt");
        fs::write(&untracked, "new").unwrap();

        let clean = tmp.path().join("clean.txt");
        commit_file(&repo, &clean, "stays clean");

        let statuses = get_file_statuses(tmp.path()).unwrap();
        assert_eq!(statuses.get(&tracked), Some(&GitFileStatus::Modified));
        assert_eq!(statuses.get(&untracked), Some(&GitFileStatus::Untracked));
        let clean_status = statuses.get(&clean);
        assert!(clean_status.is_none() || clean_status == Some(&GitFileStatus::Unchanged),);
    }

    #[test]
    fn get_file_statuses_detects_staged_file() {
        let (tmp, repo) = setup_git_repo();
        let file_path = tmp.path().join("staged.txt");
        fs::write(&file_path, "new file").unwrap();

        // Stage but don't commit
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("staged.txt")).unwrap();
        index.write().unwrap();

        let statuses = get_file_statuses(tmp.path()).unwrap();
        assert_eq!(statuses.get(&file_path), Some(&GitFileStatus::Added));
    }

    // --- get_status_for_path tests ---

    #[test]
    fn get_status_for_path_existing_returns_status() {
        let (tmp, repo) = setup_git_repo();
        let file_path = tmp.path().join("file.txt");
        commit_file(&repo, &file_path, "original");
        fs::write(&file_path, "changed").unwrap();

        let statuses = get_file_statuses(tmp.path()).unwrap();
        let status = get_status_for_path(&statuses, &file_path);
        assert_eq!(status, GitFileStatus::Modified);
    }

    #[test]
    fn get_status_for_path_missing_returns_unchanged() {
        let statuses = HashMap::new();
        let status = get_status_for_path(&statuses, Path::new("/nonexistent"));
        assert_eq!(status, GitFileStatus::Unchanged);
    }

    #[test]
    fn get_status_for_path_clean_file_returns_unchanged() {
        let (tmp, repo) = setup_git_repo();
        let file_path = tmp.path().join("clean.txt");
        commit_file(&repo, &file_path, "committed");

        let statuses = get_file_statuses(tmp.path()).unwrap();
        let status = get_status_for_path(&statuses, &file_path);
        assert_eq!(status, GitFileStatus::Unchanged);
    }

    // --- GitFileStatus trait tests ---

    #[test]
    fn git_file_status_clone_and_eq() {
        let status = GitFileStatus::Modified;
        let cloned = status;
        assert_eq!(status, cloned);
    }

    #[test]
    fn git_file_status_debug_format() {
        let status = GitFileStatus::Added;
        let debug = format!("{:?}", status);
        assert!(debug.contains("Added"));
    }

    #[test]
    fn git_file_status_all_variants_distinct() {
        let variants = [
            GitFileStatus::Modified,
            GitFileStatus::Added,
            GitFileStatus::Deleted,
            GitFileStatus::Untracked,
            GitFileStatus::Unchanged,
            GitFileStatus::Renamed,
            GitFileStatus::Ignored,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    // --- map_git2_status tests ---

    #[test]
    fn map_status_wt_modified() {
        assert_eq!(
            map_git2_status(git2::Status::WT_MODIFIED),
            GitFileStatus::Modified
        );
    }

    #[test]
    fn map_status_index_modified() {
        assert_eq!(
            map_git2_status(git2::Status::INDEX_MODIFIED),
            GitFileStatus::Modified
        );
    }

    #[test]
    fn map_status_index_new() {
        assert_eq!(
            map_git2_status(git2::Status::INDEX_NEW),
            GitFileStatus::Added
        );
    }

    #[test]
    fn map_status_wt_new() {
        assert_eq!(
            map_git2_status(git2::Status::WT_NEW),
            GitFileStatus::Untracked
        );
    }

    #[test]
    fn map_status_wt_deleted() {
        assert_eq!(
            map_git2_status(git2::Status::WT_DELETED),
            GitFileStatus::Deleted
        );
    }

    #[test]
    fn map_status_index_deleted() {
        assert_eq!(
            map_git2_status(git2::Status::INDEX_DELETED),
            GitFileStatus::Deleted
        );
    }

    #[test]
    fn map_status_index_renamed() {
        assert_eq!(
            map_git2_status(git2::Status::INDEX_RENAMED),
            GitFileStatus::Renamed
        );
    }

    #[test]
    fn map_status_wt_renamed() {
        assert_eq!(
            map_git2_status(git2::Status::WT_RENAMED),
            GitFileStatus::Renamed
        );
    }

    #[test]
    fn map_status_ignored() {
        assert_eq!(
            map_git2_status(git2::Status::IGNORED),
            GitFileStatus::Ignored
        );
    }

    #[test]
    fn map_status_current() {
        assert_eq!(
            map_git2_status(git2::Status::CURRENT),
            GitFileStatus::Unchanged
        );
    }

    #[test]
    fn map_status_wt_typechange() {
        assert_eq!(
            map_git2_status(git2::Status::WT_TYPECHANGE),
            GitFileStatus::Modified
        );
    }

    #[test]
    fn map_status_index_typechange() {
        assert_eq!(
            map_git2_status(git2::Status::INDEX_TYPECHANGE),
            GitFileStatus::Modified
        );
    }
}
