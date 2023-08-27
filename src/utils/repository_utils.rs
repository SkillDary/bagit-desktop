use std::{env, path::Path};

use git2::{Cred, RemoteCallbacks, Repository};
use regex::Regex;

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

    pub fn create_new_folder_path(url: &str, location: &str) -> String {
        let mut new_folder_name = RepositoryUtils::get_folder_name_from_os(url);

        let replaced_text = &new_folder_name.replace(".git", "");
        new_folder_name = replaced_text.trim().to_owned();

        return format!("{}/{}", &location, new_folder_name);
    }

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
}
