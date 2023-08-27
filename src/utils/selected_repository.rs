/* selected_repository.rs
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

use git2::Repository;
use std::fmt;
use uuid::Uuid;

use crate::models::bagit_repository::BagitRepository;

pub struct SelectedRepository {
    pub user_repository: BagitRepository,
    pub git_repository: Option<Repository>,
}

impl fmt::Debug for SelectedRepository {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Hi")
    }
}

impl Default for SelectedRepository {
    fn default() -> Self {
        return SelectedRepository {
            user_repository: BagitRepository::new(
                Uuid::new_v4(),
                String::new(),
                String::new(),
                None,
            ),
            git_repository: None,
        };
    }
}

impl SelectedRepository {
    /**
     * Try creating a new SelectedRepository. Return an error if the repo cannot be openned.
     */
    pub fn try_fetching_selected_repository(
        repository: &BagitRepository,
    ) -> Result<SelectedRepository, git2::Error> {
        match Repository::open(&repository.path) {
            Ok(repo) => Ok(SelectedRepository {
                user_repository: repository.clone(),
                git_repository: Some(repo),
            }),
            Err(e) => Err(e),
        }
    }

    pub fn new(
        user_repository: BagitRepository,
        git_repository: Option<Repository>,
    ) -> SelectedRepository {
        return SelectedRepository {
            user_repository,
            git_repository,
        };
    }
}
