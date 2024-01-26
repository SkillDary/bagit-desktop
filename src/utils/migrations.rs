/* migrations.rs
 *
 * Copyright 2024 SkillDary
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

use rusqlite_migration::{Migrations, M};

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        // 0.1.0
        M::up(
            "CREATE TABLE IF NOT EXISTS repository (
                repositoryId TEXT PRIMARY KEY,
                name TEXT, 
                path TEXT,
                lastOpening TEXT,
                gitProfileId TEXT,
                last_fetch_commits_to_pull INTEGER,
                last_fetch_commits_to_push INTEGER
            );",
        ),
        // 0.1.0
        M::up(
            "CREATE TABLE IF NOT EXISTS gitProfile (
                profileId TEXT PRIMARY KEY,
                profileName TEXT,
                email TEXT,
                username TEXT,
                password TEXT,
                privateKeyPath TEXT,
                signingKey TEXT
            );",
        ),
        // 0.2.0
        M::up("ALTER TABLE repository DROP COLUMN last_fetch_commits_to_pull;"),
        // 0.2.0
        M::up("ALTER TABLE repository DROP COLUMN last_fetch_commits_to_push;"),
        // In the future, add more migrations here:
        //M::up("ALTER TABLE ... ADD COLUMN ... TEXT;"),
    ])
}
