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
    /// Initializes the app database.
    /// Returns a new AppDatabase.
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
                lastOpening TEXT,
                gitProfileId TEXT,
                last_fetch_commits_to_pull INTEGER,
                last_fetch_commits_to_push INTEGER
            );
            CREATE TABLE IF NOT EXISTS gitProfile (
                profileId TEXT PRIMARY KEY,
                profileName TEXT,
                email TEXT,
                username TEXT,
                password TEXT,
                privateKeyPath TEXT,
                signingKey TEXT
            );";

        connection.execute(query).unwrap();

        return AppDatabase { connection };
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

    /// Used to get recent repositories (a maximum number of 3 repositories, ordered by last opening datetime)
    pub fn get_recent_repositories(&self) -> Vec<BagitRepository> {
        let query: &str = "SELECT * FROM repository ORDER BY lastOpening DESC LIMIT 3;";
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
     * Used for getting all repositories.
     */
    pub fn get_all_repositories_with_search(&self, search: &str) -> Vec<BagitRepository> {
        let query: &str = &format!(
            "SELECT * FROM repository WHERE name LIKE '%{}%' OR path LIKE '%{}%';",
            search, search
        );
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
            let signing_key: String = statement.read::<String, _>("signingKey").unwrap();

            let uuid: Uuid = Uuid::parse_str(&id).unwrap();

            profiles.push(BagitGitProfile::new(
                uuid,
                profile_name,
                email,
                username,
                password,
                private_key_path,
                signing_key,
            ));
        }

        return profiles;
    }
    /**
     * Used for adding a new repository.
     */
    pub fn add_repository(&self, repository: &BagitRepository) {
        let query: String = if repository.git_profile_id.is_some() {
            format!(
                "INSERT INTO repository VALUES ('{}', '{}', '{}', datetime('now'), '{}');",
                repository.repository_id.to_string(),
                repository.name.replace("'", "''"),
                repository.path.replace("'", "''"),
                repository.git_profile_id.unwrap().to_string()
            )
        } else {
            format!(
                "INSERT INTO repository(repositoryId, name, path, lastOpening) VALUES ('{}', '{}', '{}', datetime('now'));",
                repository.repository_id.to_string(),
                repository.name.replace("'", "''"),
                repository.path.replace("'", "''")
            )
        };

        self.connection.execute(query).unwrap();
    }

    /// Used to update the profile used with a repository.
    pub fn change_git_profile_of_repository(&self, repo_id: Uuid, profile_id: Option<Uuid>) {
        let query: String = if profile_id.is_some() {
            format!(
                "UPDATE repository SET
            gitProfileId='{}' 
            WHERE repositoryId='{}';",
                profile_id.unwrap(),
                repo_id
            )
        } else {
            format!(
                "UPDATE repository SET
            gitProfileId=NULL 
            WHERE repositoryId='{}';",
                repo_id
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
        signing_key: &str,
    ) -> bool {
        let query: String = format!(
            "SELECT profileId FROM gitProfile WHERE profileId='{}'
            AND profileName='{}'
            AND email='{}'
            AND username='{}'
            AND password='{}'
            AND privateKeyPath='{}'
            AND signingKey='{}';",
            profile_id,
            profile_name.replace("'", "''"),
            email.replace("'", "''"),
            username.replace("'", "''"),
            password.replace("'", "''"),
            private_key_path.replace("'", "''"),
            signing_key.replace("'", "''")
        );
        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        let mut total: i32 = 0;
        while let Ok(State::Row) = statement.next() {
            total += 1;
        }

        return total != 0;
    }

    pub fn get_repository_from_path(&self, path: &str) -> Option<BagitRepository> {
        let query: String = format!(
            "SELECT * FROM repository WHERE path='{}'",
            path.replace("'", "''")
        );
        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        if let Ok(State::Row) = statement.next() {
            let id: String = statement.read::<String, _>("repositoryId").unwrap();
            let name: String = statement.read::<String, _>("name").unwrap();
            let path: String = statement.read::<String, _>("path").unwrap();
            let git_profile_id: Option<String> =
                statement.read::<Option<String>, _>("gitProfileId").unwrap();

            let git_profile_uuid: Option<Uuid> = if git_profile_id.is_some() {
                Some(Uuid::parse_str(&git_profile_id.unwrap()).unwrap())
            } else {
                None
            };

            let uuid: Uuid = Uuid::parse_str(&id).unwrap();

            return Some(BagitRepository::new(uuid, name, path, git_profile_uuid));
        } else {
            return None;
        }
    }

    /// Used to retrieve a profile by using his id.
    pub fn get_git_profile_from_id(&self, profile_id: Uuid) -> Option<BagitGitProfile> {
        let query: String = format!("SELECT * FROM gitProfile WHERE profileId='{}'", profile_id);
        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        if let Ok(State::Row) = statement.next() {
            let id: String = statement.read::<String, _>("profileId").unwrap();
            let email: String = statement.read::<String, _>("email").unwrap();
            let profile_name: String = statement.read::<String, _>("profileName").unwrap();
            let username: String = statement.read::<String, _>("username").unwrap();
            let password: String = statement.read::<String, _>("password").unwrap();
            let private_key_path: String = statement.read::<String, _>("privateKeyPath").unwrap();
            let signing_key: String = statement.read::<String, _>("signingKey").unwrap();

            let uuid: Uuid = Uuid::parse_str(&id).unwrap();

            return Some(BagitGitProfile::new(
                uuid,
                profile_name,
                email,
                username,
                password,
                private_key_path,
                signing_key,
            ));
        } else {
            return None;
        }
    }

    /// Used to retrieve a profile by using his name.
    pub fn get_git_profile_from_name(&self, profile_name: &str) -> Option<BagitGitProfile> {
        let query: String = format!(
            "SELECT * FROM gitProfile WHERE profileName='{}'",
            profile_name.replace("'", "''")
        );
        let mut statement: sqlite::Statement<'_> = self.connection.prepare(query).unwrap();

        if let Ok(State::Row) = statement.next() {
            let id: String = statement.read::<String, _>("profileId").unwrap();
            let email: String = statement.read::<String, _>("email").unwrap();
            let profile_name: String = statement.read::<String, _>("profileName").unwrap();
            let username: String = statement.read::<String, _>("username").unwrap();
            let password: String = statement.read::<String, _>("password").unwrap();
            let private_key_path: String = statement.read::<String, _>("privateKeyPath").unwrap();
            let signing_key: String = statement.read::<String, _>("signingKey").unwrap();

            let uuid: Uuid = Uuid::parse_str(&id).unwrap();

            return Some(BagitGitProfile::new(
                uuid,
                profile_name,
                email,
                username,
                password,
                private_key_path,
                signing_key,
            ));
        } else {
            return None;
        }
    }

    /**
     * Used for adding a new git profile.
     */
    pub fn add_git_profile(&self, profile: &BagitGitProfile) {
        let query: String = format!(
            "INSERT INTO gitProfile VALUES ('{}', '{}', '{}', '{}', '{}', '{}', '{}');",
            profile.profile_id.to_string(),
            profile.profile_name.replace("'", "''"),
            profile.email.replace("'", "''"),
            profile.username.replace("'", "''"),
            profile.password.replace("'", "''"),
            profile.private_key_path.replace("'", "''"),
            profile.signing_key.replace("'", "''")
        );

        self.connection.execute(query).unwrap();
    }

    /**
     * Used for updating a git profile.
     */
    pub fn update_git_profile(&self, profile: &BagitGitProfile) {
        let query: String = format!(
            "UPDATE gitProfile SET
            profileName='{}',
            email='{}',
            username='{}',
            password='{}',
            privateKeyPath='{}',
            signingKey='{}'
            WHERE profileId='{}';",
            profile.profile_name.replace("'", "''"),
            profile.email.replace("'", "''"),
            profile.username.replace("'", "''"),
            profile.password.replace("'", "''"),
            profile.private_key_path.replace("'", "''"),
            profile.signing_key.replace("'", "''"),
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
     * Used to delete a repository.
     */
    pub fn delete_repository(&self, repository_id: &str) {
        let query: String = format!(
            "DELETE FROM repository WHERE repositoryId='{}';",
            repository_id
        );
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
            profile_name.replace("'", "''"),
            profile_id
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

    /// Used to update the last opening date of a repository.
    pub fn update_last_opening_of_repository(&self, repository_id: Uuid) {
        let query: String = format!(
            "UPDATE repository SET
            lastOpening=datetime('now') WHERE repositoryId='{}';",
            repository_id
        );

        self.connection.execute(query).unwrap();
    }

    /// Check for deleted repositories on the device.
    /// If deleted repo were found, we return their ids.
    pub fn check_for_deleted_repositories(&self) -> Vec<Uuid> {
        let all_repo = self.get_all_repositories();
        let mut deleted_ids: Vec<Uuid> = vec![];

        for repo in all_repo {
            match git2::Repository::open(&repo.path) {
                Ok(_) => {}
                Err(_) => {
                    self.delete_repository(&repo.repository_id.to_string());
                    deleted_ids.push(repo.repository_id)
                }
            }
        }

        return deleted_ids;
    }
}
