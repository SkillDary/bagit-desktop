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
// use sqlite::{Connection, State};
use rusqlite::{Connection, OptionalExtension, Statement};
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
    pub fn init_database() -> AppDatabase {
        // Location of the project folder depending on the OS.
        let project_dir: ProjectDirs =
            ProjectDirs::from("com", "SkillDary", "Bagit Desktop").unwrap();

        // The path into which we will save the database.
        let db_dir: std::path::PathBuf = project_dir.data_dir().join("db");

        // Create the necessary folders if needed.
        std::fs::create_dir_all(&db_dir).unwrap();

        let connection: Connection = rusqlite::Connection::open(db_dir.join("bagit.db")).unwrap();
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

        connection.execute(query, []).unwrap();

        return AppDatabase { connection };
    }

    /// Retrieves all repositories.
    pub fn get_all_repositories(&self) -> Result<Vec<BagitRepository>, rusqlite::Error> {
        let query: &str = "SELECT * FROM repository;";
        let mut repositories: Vec<BagitRepository> = Vec::new();

        let mut statement: Statement = self.connection.prepare(query)?;

        let repository_iter = statement.query_map([], |row| {
            let id: String = row.get("repositoryId")?;

            let git_profile_id: Option<String> = row.get("gitProfileId")?;

            let git_profile_uuid: Option<Uuid> = if git_profile_id.is_some() {
                Some(Uuid::parse_str(&git_profile_id.unwrap()).unwrap())
            } else {
                None
            };

            Ok(BagitRepository {
                repository_id: Uuid::parse_str(&id).unwrap(),
                name: row.get("name")?,
                path: row.get("path")?,
                git_profile_id: git_profile_uuid,
            })
        })?;

        for repository in repository_iter {
            repositories.push(repository?);
        }

        return Ok(repositories);
    }

    /// Retrieves recent repositories (a maximum of 3 repositories, sorted by last opened date).
    pub fn get_recent_repositories(&self) -> Result<Vec<BagitRepository>, rusqlite::Error> {
        let query: &str = "SELECT * FROM repository ORDER BY lastOpening DESC LIMIT 3;";
        let mut repositories: Vec<BagitRepository> = Vec::new();

        let mut statement: rusqlite::Statement = self.connection.prepare(query)?;

        let repository_iter = statement
            .query_map([], |row| {
                let id: String = row.get("repositoryId")?;

                let git_profile_id: Option<String> = row.get("gitProfileId")?;

                let git_profile_uuid: Option<Uuid> = if git_profile_id.is_some() {
                    Some(Uuid::parse_str(&git_profile_id.unwrap()).unwrap())
                } else {
                    None
                };

                Ok(BagitRepository {
                    repository_id: Uuid::parse_str(&id).unwrap(),
                    name: row.get("name")?,
                    path: row.get("path")?,
                    git_profile_id: git_profile_uuid,
                })
            })
            .unwrap();

        for repository in repository_iter {
            repositories.push(repository?);
        }

        return Ok(repositories);
    }

    /// Retrieves all repositories, filtered by a search string.
    pub fn get_all_repositories_with_search(
        &self,
        search: &str,
    ) -> Result<Vec<BagitRepository>, rusqlite::Error> {
        let mut repositories: Vec<BagitRepository> = Vec::new();

        let query = "SELECT * FROM repository WHERE name LIKE '%' || ?1 || '%' OR path LIKE '%' || ?2 || '%';";

        let parameters = [search, search];

        let mut statement: rusqlite::Statement = self.connection.prepare(query)?;

        let repository_iter = statement.query_map(parameters, |row| {
            let id: String = row.get("repositoryId")?;

            let git_profile_id: Option<String> = row.get("gitProfileId")?;

            let git_profile_uuid: Option<Uuid> = if git_profile_id.is_some() {
                Some(Uuid::parse_str(&git_profile_id.unwrap()).unwrap())
            } else {
                None
            };

            Ok(BagitRepository {
                repository_id: Uuid::parse_str(&id).unwrap(),
                name: row.get("name")?,
                path: row.get("path")?,
                git_profile_id: git_profile_uuid,
            })
        })?;

        for repository in repository_iter {
            repositories.push(repository?);
        }

        return Ok(repositories);
    }

    /// Retrieves all Git profiles.
    pub fn get_all_git_profiles(&self) -> Result<Vec<BagitGitProfile>, rusqlite::Error> {
        let mut profiles: Vec<BagitGitProfile> = Vec::new();

        let query: &str = "SELECT * FROM gitProfile;";

        let mut statement: rusqlite::Statement = self.connection.prepare(query)?;

        let profile_iter = statement.query_map([], |row| {
            let profile_id: Option<String> = row.get("profileId")?;

            let profile_uuid: Option<Uuid> = if profile_id.is_some() {
                Some(Uuid::parse_str(&profile_id.unwrap()).unwrap())
            } else {
                None
            };

            Ok(BagitGitProfile {
                profile_id: profile_uuid.unwrap(),
                profile_name: row.get("profileName")?,
                email: row.get("email")?,
                username: row.get("username")?,
                password: row.get("password")?,
                private_key_path: row.get("privateKeyPath")?,
                signing_key: row.get("signingKey")?,
            })
        })?;

        for profile in profile_iter {
            profiles.push(profile?);
        }

        return Ok(profiles);
    }

    /// Adds a new repository.
    pub fn add_repository(&self, repository: &BagitRepository) -> Result<(), rusqlite::Error> {
        if repository.git_profile_id.is_some() {
            let query = "INSERT INTO repository VALUES (?1, ?2, ?3, datetime('now'), ?4);";

            let parameters = [
                repository.repository_id.to_string(),
                repository.name.to_owned(),
                repository.path.to_owned(),
                repository.git_profile_id.unwrap().to_string(),
            ];

            self.connection.execute(query, parameters)?;
        } else {
            let query = "INSERT INTO repository(repositoryId, name, path, lastOpening) VALUES (?1, ?2, ?3, datetime('now'));";

            let parameters = [
                repository.repository_id.to_string(),
                repository.name.to_owned(),
                repository.path.to_owned(),
            ];

            self.connection.execute(query, parameters)?;
        }

        Ok(())
    }

    /// Updates the profile used with a repository.
    pub fn change_git_profile_of_repository(
        &self,
        repo_id: Uuid,
        profile_id: Option<Uuid>,
    ) -> Result<(), rusqlite::Error> {
        if profile_id.is_some() {
            let query = "UPDATE repository SET gitProfileId=?1 WHERE repositoryId=?2;";

            let parameters = [profile_id.unwrap().to_string(), repo_id.to_string()];

            self.connection.execute(query, parameters)?;
        } else {
            let query = "UPDATE repository SET gitProfileId=NULL WHERE repositoryId=?1;";

            let parameters = [repo_id.to_string()];

            self.connection.execute(query, parameters)?;
        }

        Ok(())
    }

    /// Checks if a Git profile already exist.
    pub fn does_git_profile_exist(&self, profile_id: &str) -> Result<bool, rusqlite::Error> {
        let query = "SELECT profileId FROM gitProfile WHERE profileId=?1;";

        let parameters = [profile_id];

        let mut statement: rusqlite::Statement<'_> = self.connection.prepare(query)?;

        return statement.exists(parameters);
    }

    /// Checks whether a Git profile already exists with the same information.
    pub fn does_git_profile_exist_from_information(
        &self,
        profile_id: &str,
        profile_name: &str,
        email: &str,
        username: &str,
        password: &str,
        private_key_path: &str,
        signing_key: &str,
    ) -> Result<bool, rusqlite::Error> {
        let query = "SELECT profileId FROM gitProfile WHERE profileId=?1
            AND profileName=?2
            AND email=?3
            AND username=?4
            AND password=?5
            AND privateKeyPath=?6
            AND signingKey=?7;";

        let parameters = [
            profile_id,
            profile_name,
            email,
            username,
            password,
            private_key_path,
            signing_key,
        ];

        let mut statement: rusqlite::Statement<'_> = self.connection.prepare(query)?;

        return statement.exists(parameters);
    }

    /// Retrieves a repository using its path.
    pub fn get_repository_from_path(
        &self,
        path: &str,
    ) -> Result<Option<BagitRepository>, rusqlite::Error> {
        let query = "SELECT * FROM repository WHERE path=?";

        let parameters = [path];

        let mut statement: rusqlite::Statement = self.connection.prepare(query)?;

        let bagit_repository = statement
            .query_row(parameters, |row| {
                let id: String = row.get("repositoryId")?;
                let uuid: Uuid = Uuid::parse_str(&id).unwrap();

                let git_profile_id: Option<String> = row.get("gitProfileId")?;

                let git_profile_uuid: Option<Uuid> = if git_profile_id.is_some() {
                    Some(Uuid::parse_str(&git_profile_id.unwrap()).unwrap())
                } else {
                    None
                };

                Ok(BagitRepository {
                    repository_id: uuid,
                    name: row.get("name")?,
                    path: row.get("path")?,
                    git_profile_id: git_profile_uuid,
                })
            })
            .optional()?;

        return Ok(bagit_repository);
    }

    /// Retrieves a profile using its ID.
    pub fn get_git_profile_from_id(
        &self,
        profile_id: Uuid,
    ) -> Result<Option<BagitGitProfile>, rusqlite::Error> {
        let query = "SELECT * FROM gitProfile WHERE profileId=?";

        let parameters = [profile_id.to_string()];

        let mut statement: rusqlite::Statement = self.connection.prepare(query)?;

        let git_profile = statement
            .query_row(parameters, |row| {
                Ok(BagitGitProfile {
                    profile_id,
                    profile_name: row.get("profileName")?,
                    email: row.get("email")?,
                    username: row.get("username")?,
                    password: row.get("password")?,
                    private_key_path: row.get("privateKeyPath")?,
                    signing_key: row.get("signingKey")?,
                })
            })
            .optional()?;

        return Ok(git_profile);
    }

    /// Retrieves a profile using its name.
    pub fn get_git_profile_from_name(
        &self,
        profile_name: &str,
    ) -> Result<Option<BagitGitProfile>, rusqlite::Error> {
        let query = "SELECT * FROM gitProfile WHERE profileName=?";

        let parameters = [profile_name];

        let mut statement: rusqlite::Statement = self.connection.prepare(query)?;

        let git_profile = statement
            .query_row(parameters, |row| {
                let id: String = row.get("profileId")?;
                let uuid: Uuid = Uuid::parse_str(&id).unwrap();

                Ok(BagitGitProfile {
                    profile_id: uuid,
                    profile_name: profile_name.to_string(),
                    email: row.get("email")?,
                    username: row.get("username")?,
                    password: row.get("password")?,
                    private_key_path: row.get("privateKeyPath")?,
                    signing_key: row.get("signingKey")?,
                })
            })
            .optional()?;

        return Ok(git_profile);
    }

    /// Adds a new Git profile.
    pub fn add_git_profile(&self, profile: &BagitGitProfile) -> Result<(), rusqlite::Error> {
        let query = "INSERT INTO gitProfile VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);";

        let parameters = [
            profile.profile_id.to_string(),
            profile.profile_name.to_owned(),
            profile.email.to_owned(),
            profile.username.to_owned(),
            profile.password.to_owned(),
            profile.private_key_path.to_owned(),
            profile.signing_key.to_owned(),
        ];

        self.connection.execute(query, parameters)?;

        Ok(())
    }

    /// Updates a Git profile.
    pub fn update_git_profile(&self, profile: &BagitGitProfile) -> Result<(), rusqlite::Error> {
        let query = "UPDATE gitProfile SET 
            profileName=?1,
            email=?2,
            username=?3,
            password=?4,
            privateKeyPath=?5,
            signingKey=?6
            WHERE profileId=?7;";

        let parameters = [
            profile.profile_name.to_owned(),
            profile.email.to_owned(),
            profile.username.to_owned(),
            profile.password.to_owned(),
            profile.private_key_path.to_owned(),
            profile.signing_key.to_owned(),
            profile.profile_id.to_string(),
        ];

        self.connection.execute(query, parameters)?;

        Ok(())
    }

    /// Deletes a Git profile.
    pub fn delete_git_profile(&self, profile_id: &str) -> Result<(), rusqlite::Error> {
        let query = "DELETE FROM gitProfile WHERE profileId=?1;";

        let parameters = [profile_id];

        self.connection.execute(query, parameters)?;

        Ok(())
    }

    /// Deletes a repository.
    pub fn delete_repository(&self, repository_id: &str) -> Result<(), rusqlite::Error> {
        let query = "DELETE FROM repository WHERE repositoryId=?1;";

        let parameters = [repository_id];

        self.connection.execute(query, parameters)?;

        Ok(())
    }

    /// Retrieve names of Git profiles with similar names to the given profile, excluding the
    /// current profile.
    pub fn get_names_of_git_profiles_with_identical_name(
        &self,
        profile_name: &str,
        profile_id: &str,
    ) -> Result<Vec<String>, rusqlite::Error> {
        let mut profiles_names: Vec<String> = Vec::new();

        let query = "SELECT profileName FROM gitProfile WHERE profileName LIKE '%' || ?1 || '%' AND profileId!=?2;";

        let parameters = [profile_name, profile_id];

        let mut statement: rusqlite::Statement<'_> = self.connection.prepare(query)?;

        let profile_iter = statement
            .query_map(parameters, |row| Ok(row.get("profileName")?))
            .unwrap();

        for profile_name in profile_iter {
            profiles_names.push(profile_name?);
        }

        return Ok(profiles_names);
    }

    /// Checks whether a Git profile name already exists in the database.
    pub fn get_number_of_git_profiles_with_name(
        &self,
        profile_name: &str,
        profile_id: &str,
    ) -> Result<i32, rusqlite::Error> {
        let profiles_names;

        match self.get_names_of_git_profiles_with_identical_name(&profile_name, &profile_id) {
            Ok(names) => profiles_names = names,
            Err(error) => {
                return Err(error);
            }
        }

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
        return Ok(matches);
    }

    /// Updates the last opening date of a repository.
    pub fn update_last_opening_of_repository(&self, repository_id: Uuid) {
        let query = "UPDATE repository SET lastOpening=datetime('now') WHERE repositoryId=?1;";

        let parameters = [repository_id.to_string()];

        self.connection.execute(query, parameters).unwrap();
    }

    /// Checks for deleted repositories on the device.
    /// If deleted repositories have been found, we return their IDs.
    pub fn check_for_deleted_repositories(&self) -> Result<Vec<Uuid>, rusqlite::Error> {
        let all_repo = self.get_all_repositories()?;
        let mut deleted_ids: Vec<Uuid> = vec![];

        for repo in all_repo {
            match git2::Repository::open(&repo.path) {
                Ok(_) => {}
                Err(_) => {
                    self.delete_repository(&repo.repository_id.to_string())?;
                    deleted_ids.push(repo.repository_id)
                }
            }
        }

        return Ok(deleted_ids);
    }
}
