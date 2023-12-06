/* git.rs
 *
 * Copyright 2023 SkillDary
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, version 3 of the License, only.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: AGPL-3.0-only
 */

use chrono::NaiveDateTime;
use gettextrs::gettext;
use git2::{Branch, Error, FetchOptions, Oid, Reference, RemoteCallbacks, Repository};

use crate::widgets::repository::CommitObject;

use super::{clone_mode::CloneMode, fetch_result::FetchResult, repository_utils::RepositoryUtils};

fn commit_to_commit_object(commit: git2::Commit, is_pushed: bool) -> CommitObject {
    let commit_datetime: NaiveDateTime =
        NaiveDateTime::from_timestamp_opt(commit.time().seconds(), 0).unwrap();

    let commit_subtitle: String = format!(
        "{} {}",
        commit.author().name().unwrap(),
        commit_datetime.format(&gettext("_Commit subtitle format"))
    );

    let message: String = commit
        .message()
        .expect("Unable to retrieve commit message")
        .trim()
        .to_string();

    let (title, description) = match message.split_once("\n") {
        Some((title, description)) => (title.to_string(), description.to_string()),
        None => (message, String::from("")),
    };

    return CommitObject::new(
        commit.id().to_string(),
        title,
        description,
        commit_subtitle,
        is_pushed,
    );
}

/// Loads n commits, either the first n commits or
/// the next n commits.
///
/// It limits the number of commits in order to not block the main thread, and to be
/// able to display the first commits instantly, without waiting to load every commit
/// before displaying the history.
pub fn load_commit_history(
    repository: &Repository,
    branch: Branch,
    starting_commit_id: String,
    nb_commits_to_load: i32,
) -> Vec<CommitObject> {
    let starting_commit_oid: Oid;
    let starting_upstream_commit_oid: Option<Oid>;

    // We will use the upstream branch of the local one to check later if a commit is on the upstream one.
    // But, a local branch can have no upstream branch so we must take care of this case.
    let upstream_branch_reference: Option<git2::Reference<'_>> = match branch.upstream() {
        Ok(branch) => Some(branch.into_reference()),
        Err(_) => None,
    };

    if starting_commit_id.is_empty() {
        let branch_reference: git2::Reference<'_> = branch.into_reference();

        let branch_commit: git2::AnnotatedCommit<'_> = repository
            .reference_to_annotated_commit(&branch_reference)
            .unwrap();

        starting_commit_oid = branch_commit.id();

        // We try to get the first commit of the upstream branch:
        starting_upstream_commit_oid = match &upstream_branch_reference {
            Some(upstream_reference) => Some(
                repository
                    .reference_to_annotated_commit(upstream_reference)
                    .unwrap()
                    .id(),
            ),
            None => None,
        };
    } else {
        starting_commit_oid = Oid::from_str(&starting_commit_id).expect("Invalid OID format");
        // In this situation, we try at first to position ourself in the commit Oid as the one on the local branch:
        starting_upstream_commit_oid = match &upstream_branch_reference {
            Some(_) => Some(Oid::from_str(&starting_commit_id).expect("Invalid OID format")),
            None => None,
        }
    }

    let mut revwalk: git2::Revwalk<'_> = repository.revwalk().unwrap();
    let mut upstream_revwalk: Option<git2::Revwalk<'_>> = match starting_upstream_commit_oid {
        Some(oid) => {
            // If we have an upstream commit id, we will iter on the upstram branch:
            let mut rev = repository.revwalk().unwrap();
            // We try to push the starting upstream commit to it:
            match rev.push(oid) {
                // If successful, we return the revwalk
                Ok(_) => {
                    rev.set_sorting(git2::Sort::TOPOLOGICAL).unwrap();
                    Some(rev.into_iter())
                }
                // Else, we try to go to the start of the upstream commit tree :
                Err(_) => {
                    match rev.push(
                        repository
                            .reference_to_annotated_commit(&upstream_branch_reference.unwrap())
                            .unwrap()
                            .id(),
                    ) {
                        Ok(_) => {
                            rev.set_sorting(git2::Sort::TOPOLOGICAL).unwrap();
                            Some(rev.into_iter())
                        }
                        Err(_) => None,
                    }
                }
            }
        }
        None => None,
    };

    revwalk.push(starting_commit_oid).unwrap();

    revwalk.set_sorting(git2::Sort::TOPOLOGICAL).unwrap();

    let mut commit_object_vector: Vec<CommitObject> = Vec::new();

    // We only load a maximum number of commits at a time.
    let revwalk_result = revwalk.push_range(&format!(
        "{}~{}..{}",
        starting_commit_oid, nb_commits_to_load, starting_commit_oid
    ));

    match revwalk_result {
        Ok(_) => {}
        Err(_) => {}
    }

    let upstream_revwalk_result = match upstream_revwalk {
        Some(mut rev) => {
            let result = rev.push_range(&format!(
                "{}~{}..{}",
                starting_upstream_commit_oid.unwrap(),
                nb_commits_to_load,
                starting_upstream_commit_oid.unwrap()
            ));
            upstream_revwalk = Some(rev);
            result
        }
        None => Ok(()),
    };

    match upstream_revwalk_result {
        Ok(_) => {}
        Err(_) => {}
    }

    let mut current_upstream_oid: Option<git2::Oid> = match upstream_revwalk {
        Some(mut rev) => {
            let oid: Option<Oid> = match rev.next() {
                Some(next_value) => Some(next_value.unwrap()),
                None => None,
            };
            upstream_revwalk = Some(rev);
            oid
        }
        None => None,
    };

    for commit_id in revwalk {
        if !starting_commit_id.is_empty() && commit_id == Ok(starting_commit_oid) {
            current_upstream_oid = match upstream_revwalk {
                Some(mut rev) => {
                    let oid: Option<Oid> = match rev.next() {
                        Some(next_value) => Some(next_value.unwrap()),
                        None => None,
                    };
                    upstream_revwalk = Some(rev);
                    oid
                }
                None => None,
            };
            continue;
        }

        let oid: git2::Oid = commit_id.unwrap();

        // We check if the current commit is the same as the one in the upstream branch:
        // If we don't have an upstream branch, the default result will be false (we need to push the local branch!)
        let is_same_oid_as_upstream: bool = match current_upstream_oid {
            Some(upstream_oid) => oid == upstream_oid,
            None => false,
        };

        if is_same_oid_as_upstream {
            // If we got the same commit (local and upstream), we can advance further in the upstream branch:
            current_upstream_oid = match upstream_revwalk {
                Some(mut rev) => {
                    let oid: Option<Oid> = match rev.next() {
                        Some(next_value) => Some(next_value.unwrap()),
                        None => None,
                    };
                    upstream_revwalk = Some(rev);
                    oid
                }
                None => None,
            };
        }

        let commit: git2::Commit<'_> = repository.find_commit(oid).unwrap();

        let commit_object: CommitObject =
            commit_to_commit_object(commit.clone(), is_same_oid_as_upstream);

        commit_object_vector.push(commit_object);
    }

    return commit_object_vector;
}

/// Gets repository checked out branch.
pub fn get_repository_checked_out_branch(repository: &Repository) -> Result<Reference<'_>, Error> {
    repository.head()
}

/// Gets repository checked out branch name.
pub fn get_repository_checked_out_branch_name(repository: &Repository) -> Result<String, Error> {
    let head = repository.head()?;

    let checked_out_branch: &str = match head.shorthand() {
        Some(it) => it,
        None => {
            return Err(git2::Error::new(
                git2::ErrorCode::GenericError,
                git2::ErrorClass::Reference,
                "Could not get the full shorthand of reference.",
            ))
        }
    };

    return Ok(checked_out_branch.to_string());
}

/// Fetches the checked out branch.
pub fn fetch_checked_out_branch(
    repository: &Repository,
    username: String,
    password: String,
    private_key_path: String,
    passphrase: String,
) -> Result<FetchResult, git2::Error> {
    let head = repository.head()?;

    let checked_out_branch: &str = match head.shorthand() {
        Some(it) => it,
        None => {
            return Err(git2::Error::new(
                git2::ErrorCode::GenericError,
                git2::ErrorClass::Reference,
                "Could not get the full shorthand of reference.",
            ))
        }
    };

    let branch = repository.find_branch(checked_out_branch, git2::BranchType::Local)?;

    let callback: RemoteCallbacks<'_>;

    match RepositoryUtils::get_clone_mode_of_repository(&repository) {
        Ok(clone_mode) => {
            callback = match clone_mode {
                CloneMode::SSH => {
                    RepositoryUtils::ssh_callback(username, passphrase, private_key_path)
                }
                CloneMode::HTTPS => RepositoryUtils::https_callback(username, password),
            }
        }
        Err(error) => return Err(error),
    };

    let mut fetch_options = FetchOptions::new();

    fetch_options.remote_callbacks(callback);

    repository.find_remote("origin")?.fetch(
        &["refs/heads/*:refs/remotes/origin/*"],
        Some(&mut fetch_options),
        None,
    )?;

    let upstream_branch = branch.upstream()?;
    let upstream_commit = upstream_branch.into_reference().peel_to_commit()?;
    let commit_local = branch.into_reference().peel_to_commit()?;

    let diff = repository.graph_ahead_behind(commit_local.id(), upstream_commit.id())?;

    return Ok(FetchResult {
        total_commits_to_push: diff.0 as i64,
        total_commits_to_pull: diff.1 as i64,
    });
}

/// Gets first commit of checked out branch.
pub fn get_first_commit_id_of_checked_out_branch(repository: &Repository) -> Option<git2::Oid> {
    let checked_out_branch;

    match get_repository_checked_out_branch(&repository) {
        Ok(repository_checked_out_branch) => checked_out_branch = repository_checked_out_branch,
        Err(_) => return None,
    }

    let branch_commit: git2::AnnotatedCommit<'_> = repository
        .reference_to_annotated_commit(&checked_out_branch)
        .unwrap();

    let branch_commit_id = branch_commit.id();

    let first_commit = repository.find_commit(branch_commit_id);

    match first_commit {
        Ok(commit) => return Some(commit.id()),
        Err(_) => {
            return None;
        }
    }
}

/// Gets the error code text.
pub fn _get_error_code_text(error_code: git2::ErrorCode) -> String {
    return match error_code {
        git2::ErrorCode::GenericError => gettext("_ErrorCode_GenericError"),
        git2::ErrorCode::NotFound => gettext("_ErrorCode_NotFound"),
        git2::ErrorCode::Exists => gettext("_ErrorCode_Exists"),
        git2::ErrorCode::Ambiguous => gettext("_ErrorCode_Ambiguous"),
        git2::ErrorCode::BufSize => gettext("_ErrorCode_BufSize"),
        git2::ErrorCode::User => gettext("_ErrorCode_User"),
        git2::ErrorCode::BareRepo => gettext("_ErrorCode_BareRepo"),
        git2::ErrorCode::UnbornBranch => gettext("_ErrorCode_UnbornBranch"),
        git2::ErrorCode::Unmerged => gettext("_ErrorCode_Unmerged"),
        git2::ErrorCode::NotFastForward => gettext("_ErrorCode_NotFastForward"),
        git2::ErrorCode::InvalidSpec => gettext("_ErrorCode_InvalidSpec"),
        git2::ErrorCode::Conflict => gettext("_ErrorCode_Conflict"),
        git2::ErrorCode::Locked => gettext("_ErrorCode_Locked"),
        git2::ErrorCode::Modified => gettext("_ErrorCode_Modified"),
        git2::ErrorCode::Auth => gettext("_ErrorCode_Auth"),
        git2::ErrorCode::Certificate => gettext("_ErrorCode_Certificate"),
        git2::ErrorCode::Applied => gettext("_ErrorCode_Applied"),
        git2::ErrorCode::Peel => gettext("_ErrorCode_Peel"),
        git2::ErrorCode::Eof => gettext("_ErrorCode_Eof"),
        git2::ErrorCode::Invalid => gettext("_ErrorCode_Invalid"),
        git2::ErrorCode::Uncommitted => gettext("_ErrorCode_Uncommitted"),
        git2::ErrorCode::Directory => gettext("_ErrorCode_Directory"),
        git2::ErrorCode::MergeConflict => gettext("_ErrorCode_MergeConflict"),
        git2::ErrorCode::HashsumMismatch => gettext("_ErrorCode_HashsumMismatch"),
        git2::ErrorCode::IndexDirty => gettext("_ErrorCode_IndexDirty"),
        git2::ErrorCode::ApplyFail => gettext("_ErrorCode_ApplyFail"),
        git2::ErrorCode::Owner => todo!(),
    };
}

/// Gets the error class text.
pub fn _get_error_class_text(error_class: git2::ErrorClass) -> String {
    return match error_class {
        git2::ErrorClass::None => gettext("_ErrorClass_None"),
        git2::ErrorClass::NoMemory => gettext("_ErrorClass_NoMemory"),
        git2::ErrorClass::Os => gettext("_ErrorClass_Os"),
        git2::ErrorClass::Invalid => gettext("_ErrorClass_Invalid"),
        git2::ErrorClass::Reference => gettext("_ErrorClass_Reference"),
        git2::ErrorClass::Zlib => gettext("_ErrorClass_Zlib"),
        git2::ErrorClass::Repository => gettext("_ErrorClass_Repository"),
        git2::ErrorClass::Config => gettext("_ErrorClass_Config"),
        git2::ErrorClass::Regex => gettext("_ErrorClass_Regex"),
        git2::ErrorClass::Odb => gettext("_ErrorClass_Odb"),
        git2::ErrorClass::Index => gettext("_ErrorClass_Index"),
        git2::ErrorClass::Object => gettext("_ErrorClass_Object"),
        git2::ErrorClass::Net => gettext("_ErrorClass_Net"),
        git2::ErrorClass::Tag => gettext("_ErrorClass_Tag"),
        git2::ErrorClass::Tree => gettext("_ErrorClass_Tree"),
        git2::ErrorClass::Indexer => gettext("_ErrorClass_Indexer"),
        git2::ErrorClass::Ssl => gettext("_ErrorClass_Ssl"),
        git2::ErrorClass::Submodule => gettext("_ErrorClass_Submodule"),
        git2::ErrorClass::Thread => gettext("_ErrorClass_Thread"),
        git2::ErrorClass::Stash => gettext("_ErrorClass_Stash"),
        git2::ErrorClass::Checkout => gettext("_ErrorClass_Checkout"),
        git2::ErrorClass::FetchHead => gettext("_ErrorClass_FetchHead"),
        git2::ErrorClass::Merge => gettext("_ErrorClass_Merge"),
        git2::ErrorClass::Ssh => gettext("_ErrorClass_Ssh"),
        git2::ErrorClass::Filter => gettext("_ErrorClass_Filter"),
        git2::ErrorClass::Revert => gettext("_ErrorClass_Revert"),
        git2::ErrorClass::Callback => gettext("_ErrorClass_Callback"),
        git2::ErrorClass::CherryPick => gettext("_ErrorClass_CherryPick"),
        git2::ErrorClass::Describe => gettext("_ErrorClass_Describe"),
        git2::ErrorClass::Rebase => gettext("_ErrorClass_Rebase"),
        git2::ErrorClass::Filesystem => gettext("_ErrorClass_Filesystem"),
        git2::ErrorClass::Patch => gettext("_ErrorClass_Patch"),
        git2::ErrorClass::Worktree => gettext("_ErrorClass_Worktree"),
        git2::ErrorClass::Sha1 => gettext("_ErrorClass_Sha1"),
        git2::ErrorClass::Http => gettext("_ErrorClass_Http"),
    };
}
