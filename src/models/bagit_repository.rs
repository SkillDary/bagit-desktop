/* bagit_repository.rs
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

use std::fmt;

use uuid::Uuid;

#[derive(Clone)]
pub struct BagitRepository {
    pub repository_id: Uuid,
    pub name: String,
    pub path: String,
    pub git_profile_id: Option<Uuid>,
}

impl fmt::Display for BagitRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "repo_id: {}\nname: {}\npath: {}\nprofile_id: {}",
            self.repository_id,
            self.name,
            self.path,
            match self.git_profile_id {
                Some(id) => id.to_string(),
                None => "None".to_string(),
            }
        )
    }
}

impl BagitRepository {
    pub fn new(
        repository_id: Uuid,
        name: String,
        path: String,
        git_profile_id: Option<Uuid>,
    ) -> BagitRepository {
        return BagitRepository {
            repository_id: repository_id,
            name: name,
            path: path,
            git_profile_id,
        };
    }
}
