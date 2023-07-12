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

use directories::ProjectDirs;
use regex::Regex;
use sqlite::{Connection, State};
use std::fmt;
use uuid::Uuid;

use crate::models::{bagit_git_profile::BagitGitProfile, bagit_repository::BagitRepository};

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
        // Location of the project folder depending on the OS.
        let project_dir: ProjectDirs =
            ProjectDirs::from("com", "SkillDary", "Bagit Desktop").unwrap();

        // The path into which we will save the database.
        let db_dir: std::path::PathBuf = project_dir.data_dir().join("db");

        // Create the necessary folders if needed.
        std::fs::create_dir_all(&db_dir).unwrap();

        let connection: Connection = sqlite::open(db_dir.join("bagit.db")).unwrap();
        let query: &str = "
            CREATE TABLE IF NOT EXISTS repository (
                repositoryId TEXT PRIMARY KEY,
                name TEXT, 
                path TEXT,
                gitProfileId TEXT
            );
            CREATE TABLE IF NOT EXISTS gitProfile (
                profileId TEXT PRIMARY KEY,
                profileName TEXT,
                email TEXT,
                username TEXT,
                password TEXT,
                privateKeyPath TEXT
            );";

        connection.execute(query).unwrap();

        return AppDatabase {
            connection: connection,
        };
    }

    /**
     * Used for getting all repositories.
     */
    pub fn get_all_repositories(&self) -> Vec<BagitRepository> {
        let query: &str = "SELECT * FROM repository;";
        let mut repositories: Vec<BagitRepository> = Vec::new();

        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        while let Ok(State::Row) = statement.next() {
            let id: String = statement.read::<String, _>("repositoryId").unwrap();
            let uuid: Uuid = Uuid::parse_str(&id).unwrap();

            let git_profile_id: Option<String> =
                statement.read::<Option<String>, _>("gitProfileId").unwrap();
            let git_profile_uuid: Option<Uuid> = if git_profile_id.is_some() {
                Some(Uuid::parse_str(&git_profile_id.unwrap()).unwrap())
            } else {
                None
            };

            repositories.push(BagitRepository::new(
                uuid,
                statement.read::<String, _>("name").unwrap(),
                statement.read::<String, _>("path").unwrap(),
                git_profile_uuid,
            ));
        }

        return repositories;
    }

    /**
     * Used for getting all git profiles.
     */
    pub fn get_all_git_profiles(&self) -> Vec<BagitGitProfile> {
        let query: &str = "SELECT * FROM gitProfile;";
        let mut profiles: Vec<BagitGitProfile> = Vec::new();

        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        while let Ok(State::Row) = statement.next() {
            let id: String = statement.read::<String, _>("profileId").unwrap();
            let email: String = statement.read::<String, _>("email").unwrap();
            let profile_name: String = statement.read::<String, _>("profileName").unwrap();
            let username: String = statement.read::<String, _>("username").unwrap();
            let password: String = statement.read::<String, _>("password").unwrap();
            let private_key_path: String = statement.read::<String, _>("privateKeyPath").unwrap();

            let uuid: Uuid = Uuid::parse_str(&id).unwrap();

            profiles.push(BagitGitProfile::new(
                uuid,
                profile_name,
                email,
                username,
                password,
                private_key_path,
            ));
        }

        return profiles;
    }
    /**
     * Used for adding a new repository.
     */
    pub fn add_repository(&self, name: &str, path: &str, profile_id: Option<Uuid>) {
        let new_id: Uuid = Uuid::new_v4();
        let query: String = if profile_id.is_some() {
            format!(
                "INSERT INTO repository VALUES ('{}', '{}', '{}', '{}');",
                new_id.to_string(),
                name,
                path,
                profile_id.unwrap().to_string()
            )
        } else {
            format!(
                "INSERT INTO repository(repositoryId, name, path) VALUES ('{}', '{}', '{}');",
                new_id.to_string(),
                name,
                path
            )
        };

        self.connection.execute(query).unwrap();
    }

    /**
     * Used for checking if a git profile already exist.
     */
    pub fn does_git_profile_exist(&self, profile_id: &str) -> bool {
        let query: String = format!(
            "SELECT profileId FROM gitProfile WHERE profileId='{}';",
            profile_id
        );
        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        let mut total: i32 = 0;
        while let Ok(State::Row) = statement.next() {
            total += 1;
        }

        return total != 0;
    }

    /**
     * Used for checking if a git profile already exist with all informations.
     */
    pub fn does_git_profile_exist_from_information(
        &self,
        profile_id: &str,
        profile_name: &str,
        email: &str,
        username: &str,
        password: &str,
        private_key_path: &str,
    ) -> bool {
        let query: String = format!(
            "SELECT profileId FROM gitProfile WHERE profileId='{}'
            AND profileName='{}'
            AND email='{}'
            AND username='{}'
            AND password='{}'
            AND privateKeyPath='{}';",
            profile_id, profile_name, email, username, password, private_key_path
        );
        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        let mut total: i32 = 0;
        while let Ok(State::Row) = statement.next() {
            total += 1;
        }

        return total != 0;
    }

    pub fn get_git_profile_from_name(&self, profile_name: &str) -> Option<BagitGitProfile> {
        let query: String = format!(
            "SELECT * FROM gitProfile WHERE profileName='{}'",
            profile_name
        );
        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        if let Ok(State::Row) = statement.next() {
            let id: String = statement.read::<String, _>("profileId").unwrap();
            let email: String = statement.read::<String, _>("email").unwrap();
            let profile_name: String = statement.read::<String, _>("profileName").unwrap();
            let username: String = statement.read::<String, _>("username").unwrap();
            let password: String = statement.read::<String, _>("password").unwrap();
            let private_key_path: String = statement.read::<String, _>("privateKeyPath").unwrap();

            let uuid: Uuid = Uuid::parse_str(&id).unwrap();

            return Some(BagitGitProfile::new(
                uuid,
                profile_name,
                email,
                username,
                password,
                private_key_path,
            ));
        } else {
            return None;
        }
    }

    /**
     * Used for adding a new git profile.
     */
    pub fn add_git_profile(&self, profile: BagitGitProfile) {
        let query: String = format!(
            "INSERT INTO gitProfile VALUES ('{}', '{}', '{}', '{}', '{}', '{}');",
            profile.profile_id.to_string(),
            profile.profile_name,
            profile.email,
            profile.username,
            profile.password,
            profile.private_key_path
        );

        self.connection.execute(query).unwrap();
    }

    /**
     * Used for updating a git profile.
     */
    pub fn update_git_profile(&self, profile: BagitGitProfile) {
        let query: String = format!(
            "UPDATE gitProfile SET
            profileName='{}',
            email='{}',
            username='{}',
            password='{}',
            privateKeyPath='{}'
            WHERE profileId='{}';",
            profile.profile_name,
            profile.email,
            profile.username,
            profile.password,
            profile.private_key_path,
            profile.profile_id,
        );

        self.connection.execute(query).unwrap();
    }

    /**
     * Used to delete a git profile.
     */
    pub fn delete_git_profile(&self, profile_id: &str) {
        let query: String = format!("DELETE FROM gitProfile WHERE profileId='{}';", profile_id);
        self.connection.execute(query).unwrap();
    }

    /**
     * Used to a list of profile names containing a string.
     * We make sure to select all profiles without the one we check.
     */
    pub fn get_git_profiles_names_with_name(
        &self,
        profile_name: &str,
        profile_id: &str,
    ) -> Vec<String> {
        let query: String = format!(
            "SELECT profileName FROM gitProfile WHERE profileName LIKE '%{}%' AND profileId!='{}';",
            profile_name, profile_id
        );

        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        let mut total: Vec<String> = Vec::new();
        while let Ok(State::Row) = statement.next() {
            total.push(statement.read::<String, _>("profileName").unwrap());
        }

        return total;
    }

    /**
     * Used to check if a git profile name already appears in the database.
     */
    pub fn get_number_of_git_profiles_with_name(
        &self,
        profile_name: &str,
        profile_id: &str,
    ) -> i32 {
        let profiles_names = self.get_git_profiles_names_with_name(&profile_name, &profile_id);
        let mut matches = 0;
        for name in profiles_names {
            let regex = format!(r"^{}(\s\(\d\))?$", profile_name);
            let re = Regex::new(&regex);
            match re {
                Ok(r) => {
                    if r.is_match(&name) || profile_name == name {
                        matches += 1;
                    }
                }
                Err(_) => {
                    if profile_name == name {
                        matches += 1;
                    }
                }
            }
        }
        return matches;
    }
}
