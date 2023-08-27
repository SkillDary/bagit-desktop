/* profile_mode.rs
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

use crate::models::bagit_git_profile::BagitGitProfile;
use std::fmt;

#[derive(Clone)]
pub enum ProfileMode {
    NoProfile,
    NewProfile,
    SelectedProfile(BagitGitProfile),
}

impl fmt::Debug for ProfileMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ProfileMode debug")
    }
}

impl fmt::Display for ProfileMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProfileMode::NoProfile => write!(f, "NoProfile"),
            ProfileMode::NewProfile => write!(f, "NewProfile"),
            ProfileMode::SelectedProfile(_) => write!(f, "SelectedProfile"),
        }
    }
}

impl Default for ProfileMode {
    fn default() -> Self {
        ProfileMode::NoProfile
    }
}

impl ProfileMode {
    /// Retrieve the profile mode.
    pub fn get_profile_mode(&self) -> ProfileMode {
        match self {
            ProfileMode::NoProfile => ProfileMode::NoProfile,
            ProfileMode::NewProfile => ProfileMode::NewProfile,
            ProfileMode::SelectedProfile(profile) => ProfileMode::SelectedProfile(profile.clone()),
        }
    }
}
