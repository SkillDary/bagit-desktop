/* repository_utils.rs
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

use std::{env, path::Path};

use gettextrs::gettext;
use git2::{
    Commit, Cred, Delta, DiffOptions, ErrorClass, ErrorCode, Index, ObjectType, Oid,
    RemoteCallbacks, Repository, Signature,
};
use regex::Regex;

use crate::{models::bagit_git_profile::BagitGitProfile, utils::gpg_utils::GpgUtils};

use super::changed_file::ChangedFile;

pub struct RepositoryUtils {}

impl RepositoryUtils {
    /**
     * Check whether user is using https to clone a repository.
     */
    pub fn is_using_https(url: &str) -> bool {
        let re = Regex::new(r"https://.*").unwrap();
        return re.is_match(url);
    }

    /**
     * Check whether user is using ssh to clone a repository.
     */
    pub fn is_using_ssh(url: &str) -> bool {
        return url.contains("@");
    }

    pub fn find_correct_callback(
        url: String,
        username: String,
        password: String,
        passphrase: String,
        private_key_path: String,
    ) -> RemoteCallbacks<'static> {
        if RepositoryUtils::is_using_https(&url) {
            RepositoryUtils::https_callback(username, password)
        } else {
            RepositoryUtils::ssh_callback(username, passphrase, private_key_path)
        }
    }

    /**
     * Used to create callback for https clone.
     */
    pub fn https_callback(
        profile_username: String,
        profile_password: String,
    ) -> RemoteCallbacks<'static> {
        let mut callback = RemoteCallbacks::new();

        callback.credentials(move |_url, username, _allowed_type| {
            if profile_username.is_empty() {
                return Cred::userpass_plaintext(
                    if username.is_some() {
                        username.unwrap()
                    } else {
                        ""
                    },
                    "",
                );
            } else {
                return Cred::userpass_plaintext(&profile_username, &profile_password);
            }
        });

        return callback;
    }

    /**
     * Used to create callback for ssh clone.
     */
    pub fn ssh_callback(
        profile_username: String,
        profile_passphrase: String,
        profile_private_key_path: String,
    ) -> RemoteCallbacks<'static> {
        let mut callback = RemoteCallbacks::new();

        let private_key_path_clone = profile_private_key_path.clone();

        callback.credentials(move |_url, username, _allowed_type| {
            if profile_username.is_empty() {
                // No cred will be used :
                return Cred::ssh_key(
                    if username.is_some() {
                        username.unwrap()
                    } else {
                        ""
                    },
                    None,
                    Path::new(""),
                    None,
                );
            } else {
                Cred::ssh_key(
                    if username.is_some() {
                        username.unwrap()
                    } else {
                        ""
                    },
                    None,
                    Path::new(&private_key_path_clone),
                    if profile_passphrase.is_empty() {
                        None
                    } else {
                        Some(&profile_passphrase)
                    },
                )
            }
        });

        return callback;
    }

    /**
     * Used to get the folder name of a path from OS information.
     */
    pub fn get_folder_name_from_os(path: &str) -> String {
        let os = env::consts::OS;

        // The path format changes depending on the OS.
        let folder_name = match os {
            "linux" | "macOS" | "freebsd" | "dragonfly" | "netbsd" | "openbsd" | "solaris" => {
                path.split("/").last().unwrap().to_string()
            }
            "windows" => path.split("\\").last().unwrap().to_string(),
            _ => "".to_string(),
        };
        return folder_name.replace(".git", "").trim().to_owned();
    }

    /// Used to create a new folder path.
    pub fn create_new_folder_path(url: &str, location: &str) -> String {
        let mut new_folder_name = RepositoryUtils::get_folder_name_from_os(url);

        let replaced_text = &new_folder_name.replace(".git", "");
        new_folder_name = replaced_text.trim().to_owned();

        return RepositoryUtils::build_path_of_file(&location, &new_folder_name);
    }

    /// Used to build a path of a file depending of the os.
    pub fn build_path_of_file(parent: &str, file_name: &str) -> String {
        let os = env::consts::OS;

        match os {
            "linux" | "macOS" | "freebsd" | "dragonfly" | "netbsd" | "openbsd" | "solaris" => {
                format!("{}/{}", &parent, file_name)
            }
            "windows" => format!("{}\\{}", &parent, file_name),
            _ => format!("{}/{}", &parent, file_name),
        }
    }

    /// Used to reset the git config of a repository.
    pub fn reset_git_config(repository: &Repository) -> Result<(), git2::Error> {
        let mut config = match repository.config() {
            Ok(config) => config,
            Err(error) => return Err(error),
        };

        let _ = config.remove("user.name");
        let _ = config.remove("user.email");
        let _ = config.remove("user.signingKey");
        let _ = config.remove("gpg.program");
        let _ = config.remove("commit.gpgsign");

        Ok(())
    }

    /// Used to write profile information to git config.
    pub fn override_git_config(
        repository: &Repository,
        profile: &BagitGitProfile,
    ) -> Result<(), git2::Error> {
        let mut config = match repository.config() {
            Ok(config) => config,
            Err(error) => return Err(error),
        };

        config.set_str("user.name", &profile.username)?;
        config.set_str("user.email", &profile.email)?;

        if profile.signing_key.is_empty() {
            let _ = config.remove("user.signingKey");
        } else {
            config.set_str("user.signingKey", &profile.signing_key)?;
        }

        config.set_str("gpg.program", "gpg2")?;
        config.set_bool("commit.gpgsign", !profile.signing_key.is_empty())?;

        Ok(())
    }

    /// Used to clone a repository and return the result.
    pub fn clone_repository(
        url: &str,
        new_folder_path: &str,
        callback: RemoteCallbacks,
    ) -> Result<Repository, git2::Error> {
        let mut fo = git2::FetchOptions::new();
        fo.remote_callbacks(callback);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fo);

        return builder.clone(&url.trim(), Path::new(&new_folder_path));
    }

    /// Used to find latest commit of a repository.
    pub fn find_last_commit(repo: &Repository) -> Result<Commit, git2::Error> {
        let obj = repo.head()?.resolve()?.peel(ObjectType::Commit)?;
        obj.into_commit()
            .map_err(|_| git2::Error::from_str("Couldn't find commit"))
    }

    /// Used to update a repository's index by adding new selected files to it.
    pub fn update_repository_index(
        repository: &Repository,
        selected_files: Vec<ChangedFile>,
    ) -> Result<Index, String> {
        let head_commit = match repository.head() {
            Ok(head) => head.peel_to_commit().ok(),
            Err(_) => return Err(gettext("_An error has occured")),
        };

        let head_tree = match head_commit {
            Some(ref commit) => commit.tree().ok(),
            None => return Err(gettext("_An error has occured")),
        };

        let mut diff_options = DiffOptions::new();
        diff_options
            .include_untracked(true)
            .recurse_ignored_dirs(true);

        let diff_result =
            repository.diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut diff_options));

        let diff_deltas: Vec<_> = match diff_result {
            Ok(ref diff) => diff.deltas().collect(),
            Err(_) => Vec::new(),
        };

        if diff_deltas.is_empty() {
            return Err(gettext("_An error has occured"));
        }

        let mut index = repository.index().ok().unwrap();

        for diff_delta in diff_deltas {
            let delta = diff_delta.status();

            match delta {
                Delta::Deleted => {
                    let path = diff_delta.old_file().path().unwrap();

                    // We check if the path correspond to a selected path :
                    if selected_files.iter().any(|file| {
                        let file_path = if file.parent.is_empty() {
                            file.name.clone()
                        } else {
                            RepositoryUtils::build_path_of_file(&file.parent, &file.name)
                        };

                        let delta_path = path.to_str().unwrap().to_string();

                        file_path == delta_path
                    }) {
                        match index.remove_path(path) {
                            Ok(_) => {}
                            Err(_) => return Err(gettext("_An error has occured")),
                        }
                    }
                }

                _ => {
                    let path = diff_delta.new_file().path().unwrap();

                    // We check if the path correspond to a selected path :
                    if selected_files.iter().any(|file| {
                        let file_path = if file.parent.is_empty() {
                            file.name.clone()
                        } else {
                            RepositoryUtils::build_path_of_file(&file.parent, &file.name)
                        };

                        let delta_path = path.to_str().unwrap().to_string();

                        file_path == delta_path
                    }) {
                        match index.add_path(path) {
                            Ok(_) => {}
                            Err(_) => return Err(gettext("_An error has occured")),
                        }
                    }
                }
            }
        }
        match index.write() {
            Ok(_) => Ok(index),
            Err(_) => Err(gettext("_An error has occured")),
        }
    }

    /// Used to commit files.
    pub fn commit_files(
        repository: &Repository,
        selected_files: Vec<ChangedFile>,
        message: &str,
        description: &str,
        author: &str,
        author_email: &str,
        signing_key: &str,
        passphrase: &str,
    ) -> Result<Oid, git2::Error> {
        let mut index = match RepositoryUtils::update_repository_index(repository, selected_files) {
            Ok(idx) => idx,
            Err(error_message) => {
                return Err(git2::Error::new(
                    ErrorCode::GenericError,
                    ErrorClass::Invalid,
                    error_message,
                ))
            }
        };

        let oid = index.write_tree()?;
        let author_signature = Signature::now(author, author_email)?;
        let parent_commit = RepositoryUtils::find_last_commit(&repository)?;
        let tree = repository.find_tree(oid).ok().unwrap();

        let final_message = if description.is_empty() {
            message.to_string()
        } else {
            format!("{}\n{}", message, description)
        };

        if signing_key.is_empty() {
            match repository.commit(
                Some("HEAD"),
                &author_signature,
                &author_signature,
                &final_message,
                &tree,
                &[&parent_commit],
            ) {
                Ok(commit_oid) => {
                    return Ok(commit_oid);
                }
                Err(error) => {
                    match repository.reset(&parent_commit.as_object(), git2::ResetType::Soft, None)
                    {
                        Ok(_) => Err(error),
                        Err(e) => Err(e),
                    }
                }
            }
        } else {
            match repository.commit_create_buffer(
                &author_signature,
                &author_signature,
                &final_message,
                &tree,
                &[&parent_commit],
            ) {
                Ok(buffer) => {
                    let commit_as_str = std::str::from_utf8(&buffer).unwrap().to_string();

                    let sig = GpgUtils::sign_commit_string_with_passphrase(
                        &commit_as_str,
                        signing_key,
                        passphrase,
                    );

                    match sig {
                        Ok(string_sig) => {
                            match repository.commit_signed(&commit_as_str, &string_sig, None) {
                                Ok(commit_oid) => {
                                    // Strangely, commit_signed will not update the HEAD. We need to do it manually
                                    let head = repository.head()?;
                                    let branch = head.shorthand();
                                    match branch {
                                        Some(branch) => {
                                            repository.reference(
                                                &format!("refs/heads/{}", branch),
                                                commit_oid,
                                                true,
                                                &message,
                                            )?;
                                        }
                                        None => {
                                            return Err(git2::Error::new(
                                                ErrorCode::GenericError,
                                                ErrorClass::Invalid,
                                                gettext("_An error has occured"),
                                            ));
                                        }
                                    }

                                    return Ok(commit_oid);
                                }
                                Err(error) => {
                                    match repository.reset(
                                        &parent_commit.as_object(),
                                        git2::ResetType::Soft,
                                        None,
                                    ) {
                                        Ok(_) => Err(error),
                                        Err(e) => Err(e),
                                    }
                                }
                            }
                        }
                        Err(error) => Err(git2::Error::new(
                            ErrorCode::GenericError,
                            ErrorClass::Invalid,
                            error,
                        )),
                    }
                }
                Err(e) => Err(e),
            }
        }
    }
}
