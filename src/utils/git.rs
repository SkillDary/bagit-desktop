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

fn commit_to_commit_object(commit: git2::Commit) -> CommitObject {
    let commit_datetime: NaiveDateTime =
        NaiveDateTime::from_timestamp_opt(commit.time().seconds(), 0).unwrap();

    let commit_subtitle: String = format!(
        "{} {}",
        commit.author().name().unwrap(),
        commit_datetime.format(&gettext("_Commit subtitle format"))
    );

    return CommitObject::new(
        commit.id().to_string(),
        commit
            .message()
            .expect("Unable to retrieve commit message")
            .trim()
            .to_string(),
        commit_subtitle,
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

    if starting_commit_id.is_empty() {
        let branch_reference: git2::Reference<'_> = branch.into_reference();

        let branch_commit: git2::AnnotatedCommit<'_> = repository
            .reference_to_annotated_commit(&branch_reference)
            .unwrap();

        starting_commit_oid = branch_commit.id();
    } else {
        starting_commit_oid = Oid::from_str(&starting_commit_id).expect("Invalid OID format");
    }

    let mut revwalk: git2::Revwalk<'_> = repository.revwalk().unwrap();
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

    for commit_id in revwalk {
        if !starting_commit_id.is_empty() && commit_id == Ok(starting_commit_oid) {
            continue;
        }

        let commit: git2::Commit<'_> = repository.find_commit(commit_id.unwrap()).unwrap();

        let commit_object: CommitObject = commit_to_commit_object(commit.clone());

        commit_object_vector.push(commit_object);
    }

    return commit_object_vector;
}
