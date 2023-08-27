/* changed_folder.rs
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

#[derive(Clone)]
pub struct ChangedFolder {
    pub path: String,
    pub is_expanded: bool,
}

impl fmt::Debug for ChangedFolder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.path, self.is_expanded)
    }
}

impl Default for ChangedFolder {
    fn default() -> Self {
        return ChangedFolder {
            path: String::new(),
            is_expanded: true,
        };
    }
}

impl fmt::Display for ChangedFolder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.path, self.is_expanded)
    }
}

impl ChangedFolder {
    /**
     * Used to create a new ChangedFolder.
     */
    pub fn new(path: String, is_expanded: bool) -> ChangedFolder {
        return ChangedFolder { path, is_expanded };
    }

    /**
     * Used to check if a folder is the same as the current one.
     */
    pub fn is_same_element(&self, changed_folder: &ChangedFolder) -> bool {
        return self.path == changed_folder.path;
    }
}
