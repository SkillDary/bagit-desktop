/* bagit_git_profile.rs
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

use crate::utils::clone_mode::CloneMode;

#[derive(Clone)]
pub struct BagitGitProfile {
    pub profile_id: Uuid,
    pub profile_name: String,
    pub email: String,
    pub username: String,
    pub password: String,
    pub private_key_path: String,
    pub signing_key: String,
}

impl fmt::Debug for BagitGitProfile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BagitGitProfile Debug")
    }
}

impl fmt::Display for BagitGitProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\nprofile_id: {}\nprofile_name: {}\nusername: {}\nprivate_key_path: {}\nsigning_key: {}", 
                self.profile_id,
            self.profile_name,
            self.username,
            self.private_key_path,
            self.signing_key
        )
    }
}

impl BagitGitProfile {
    pub fn new(
        profile_id: Uuid,
        profile_name: String,
        email: String,
        username: String,
        password: String,
        private_key_path: String,
        signing_key: String,
    ) -> BagitGitProfile {
        return BagitGitProfile {
            profile_id,
            profile_name,
            email,
            username,
            password,
            private_key_path,
            signing_key,
        };
    }

    /// Used to know if a profile has the information for actions such as pull or push.
    pub fn does_profile_has_information_for_actions(&self, clone_mode: &CloneMode) -> bool {
        match clone_mode {
            CloneMode::SSH => return !self.private_key_path.is_empty(),
            CloneMode::HTTPS => return !self.username.is_empty() && !self.password.is_empty(),
        }
    }
}
