/* db.rs
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

use sqlite::{Connection, State};
use uuid::Uuid;

use crate::models::bagit_repository::BagitRepository;

pub struct AppDatabase {
    connection: Connection,
}

impl fmt::Debug for AppDatabase {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Hi")
    }
}

impl Default for AppDatabase {
    fn default() -> Self {
        AppDatabase::init_database()
    }
}

impl AppDatabase {
    /**
     * Used to init the app database.
     * return a new AppDatabase.
     */
    pub fn init_database() -> AppDatabase {
        println!("Init !");
        let connection: Connection = sqlite::open(":appDatabase:").unwrap();
        let query: &str = "
            CREATE TABLE IF NOT EXISTS repository (
                repositoryId TEXT PRIMARY KEY,
                name TEXT, 
                path TEXT
            );
        ";

        connection.execute(query).unwrap();

        return AppDatabase {
            connection: connection,
        };
    }

    /**
     * Used for getting all repositories.
     */
    pub fn get_all_repositories(&self) -> Vec<BagitRepository> {
        let query: &str = "SELECT * FROM repository";
        let mut repositories: Vec<BagitRepository> = Vec::new();

        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        while let Ok(State::Row) = statement.next() {
            let id: String = statement.read::<String, _>("repositoryId").unwrap();
            let uuid: Uuid = Uuid::parse_str(&id).unwrap();

            repositories.push(BagitRepository::new(
                uuid,
                statement.read::<String, _>("name").unwrap(),
                statement.read::<String, _>("path").unwrap(),
            ));
        }

        return repositories;
    }

    /**
     * Used for adding a new repository.
     */
    pub fn add_repository(&self, name: &str, path: &str) {
        let new_id: Uuid = Uuid::new_v4();
        let query: String = format!(
            "INSERT INTO repository VALUES ('{}', '{}', '{}')",
            new_id.to_string(),
            name,
            path
        );

        self.connection.execute(query).unwrap();
    }
}
