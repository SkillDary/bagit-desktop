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
use git2::{Branch, Oid, Repository};

use crate::widgets::repository::CommitObject;

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
        starting_upstream_commit_oid =
            Some(Oid::from_str(&starting_commit_id).expect("Invalid OID format"));
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
