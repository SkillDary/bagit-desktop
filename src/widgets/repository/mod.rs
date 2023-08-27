/* mod.rs
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

pub mod commits_sidebar;

mod imp;

use glib::Object;
use gtk::glib::{self, object::ObjectBuilder};

glib::wrapper! {
    pub struct CommitObject(ObjectSubclass<imp::CommitObject>);
}

impl CommitObject {
    pub fn new(commit_id: String, title: String, subtitle: String) -> Self {
        let object_builder: ObjectBuilder<'_, CommitObject> = Object::builder();

        let object_builder: ObjectBuilder<'_, CommitObject> =
            object_builder.property("commit-id", commit_id);
        let object_builder: ObjectBuilder<'_, CommitObject> =
            object_builder.property("title", title);
        let object_builder: ObjectBuilder<'_, CommitObject> =
            object_builder.property("subtitle", subtitle);

        return object_builder.build();
    }
}
