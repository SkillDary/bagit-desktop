/* window.rs
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

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::utils::selected_repository::SelectedRepository;
use crate::{glib::clone, models::bagit_git_profile::BagitGitProfile, utils};
use adw::{
    subclass::prelude::*,
    traits::{ActionRowExt, PreferencesRowExt},
};
use gettextrs::gettext;
use git2::{Cred, RemoteCallbacks, Repository};
use gtk::{
    gio,
    glib::{self, MainContext},
};
use gtk::{glib::closure_local, prelude::*};
use uuid::Uuid;

use crate::clone_repository_page::BagitCloneRepositoryPage;
use crate::repository_page::BagitRepositoryPage;
use crate::widgets::action_bar::BagitActionBar;
use crate::widgets::repositories::BagitRepositories;

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/window.ui")]
    pub struct BagitDesktopWindow {
        // Template widgets
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub repositories_window: TemplateChild<BagitRepositories>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub action_bar_content: TemplateChild<BagitActionBar>,
        #[template_child]
        pub clone_repository_page: TemplateChild<BagitCloneRepositoryPage>,
        #[template_child]
        pub repository_page: TemplateChild<BagitRepositoryPage>,

        pub app_database: utils::db::AppDatabase,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BagitDesktopWindow {
        const NAME: &'static str = "BagitDesktopWindow";
        type Type = super::BagitDesktopWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitDesktopWindow {
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().connect_is_active_notify(clone!(
                @weak self as win
                => move |_| {
                match win.stack.visible_child_name().unwrap().as_str() {
                    "repository page" => win.repository_page.update_changed_files(),
                    _ => {}
                }
            }));
        }
    }
    impl WidgetImpl for BagitDesktopWindow {}
    impl WindowImpl for BagitDesktopWindow {}
    impl ApplicationWindowImpl for BagitDesktopWindow {}
    impl AdwApplicationWindowImpl for BagitDesktopWindow {}
}

glib::wrapper! {
    pub struct BagitDesktopWindow(ObjectSubclass<imp::BagitDesktopWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,        @implements gio::ActionGroup, gio::ActionMap;
}

impl BagitDesktopWindow {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        let win: BagitDesktopWindow = glib::Object::builder::<BagitDesktopWindow>()
            .property("application", application)
            .build();

        win.open_repository_signal();
        win.repository_page_signals();
        win.clone_button_signal();
        win.clone_repository_page_signals();
        win.add_existing_repository_button_signal();
        win.init_repositories();
        win
    }

    /*
     * Opens the repository the user clicked on.
     */
    pub fn open_repository_signal(&self) {
        self.imp().repositories_window.connect_closure(
            "row-selected",
            false,
            closure_local!(@watch self as win => move |
                _repository: BagitRepositories,
                path: &str
                | {
                    // We update the selected repository :
                    let found_repository = win.imp().app_database.get_repository_from_path(&path);
                    win.imp().repository_page.imp().sidebar.clear_changed_files_list();
                    win.imp().repository_page.imp().sidebar.imp().change_from_file.set(false);

                    if found_repository.is_some() {
                        win.imp().repository_page.imp().selected_repository.replace(
                            SelectedRepository::new_with_repository(&found_repository.unwrap())
                        );
                        win.imp().repository_page.update_changed_files();
                        win.imp().stack.set_visible_child_name("repository page");
                    }
                }
            ),
        );
    }

    /**
     * Used for listenning to the "Add an existing repository" button of the BagitActionBar widget.
     */
    fn add_existing_repository_button_signal(&self) {
        self.imp().action_bar_content.connect_closure(
            "add-existing-repository",
            false,
            closure_local!(@watch self as win => move |_action_bar_content: BagitActionBar| {
                let ctx: MainContext = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak win as win2 => async move {
                    let dialog = gtk::FileDialog::builder()
                        .accept_label(gettext("_Add"))
                        .modal(true)
                        .title(gettext("_Select repository"))
                        .build();

                    if let Ok(folder) = dialog.select_folder_future(Some(&win2)).await {
                        let folder_path = folder.path().unwrap_or(PathBuf::new());
                        let folder_path_str = folder_path.to_str().unwrap();

                        let repository = Repository::open(&folder_path);

                        // We make sure the selected folder is a valid repository.
                        match repository {
                            Ok(_) => {
                                let folder_name = win2.get_folder_name_from_os(folder_path_str);

                                win2.add_list_row(
                                    &folder_name,
                                    folder_path_str
                                );
                                win2.imp().app_database.add_repository(
                                    &folder_name,
                                    &folder_path_str,
                                    None
                                );
                            }
                            Err(_) => {
                                gtk::AlertDialog::builder()
                                .message(gettext("_Failed to open repository"))
                                .detail(gettext("_Selected folder is not a repository"))
                                .build()
                                .show(Some(&win2));
                            }
                        }
                    }
                }));
            }),
        );
    }

    /**
     * Add a new row to the list of all repositories.
     */
    pub fn add_list_row(&self, repo_name: &str, repo_path: &str) {
        if !self.imp().repositories_window.is_visible() {
            self.imp().status_page.set_visible(false);
            self.imp().repositories_window.set_visible(true);
        }

        let full_path: String = format!("{}{}", "~", repo_path);

        let new_row: adw::ActionRow = adw::ActionRow::new();
        let row_image: gtk::Image = gtk::Image::new();
        row_image.set_icon_name(Some("go-next-symbolic"));
        new_row.set_title(repo_name);
        new_row.set_subtitle(&full_path);
        new_row.set_height_request(64);
        new_row.add_suffix(&row_image);

        self.imp()
            .repositories_window
            .imp()
            .all_repositories
            .append(&new_row);
    }

    fn init_repositories(&self) {
        let all_repositories: Vec<crate::models::bagit_repository::BagitRepository> =
            self.imp().app_database.get_all_repositories();

        for repository in all_repositories {
            self.add_list_row(&repository.name, &repository.path)
        }
    }

    /**
     * Used for listenning to the clone button of the BagitActionBar widget.
     */
    fn clone_button_signal(&self) {
        self.imp().action_bar_content.connect_closure(
            "clone-repository",
            false,
            closure_local!(@watch self as win => move |_action_bar_content: BagitActionBar| {
                // We must make sure entry fields are blank when going to the clone page :
                win.imp().clone_repository_page.clear_page();
                // We update the list of git profiles in the page :
                let git_profiles = win.imp().app_database.get_all_git_profiles();
                for profile in git_profiles {
                    win.imp().clone_repository_page.add_git_profile_row(profile);
                }
                win.imp().stack.set_visible_child_name("clone page");
            }),
        );
    }

    fn clone_repository_page_signals(&self) {
        self.imp().clone_repository_page.connect_closure(
            "go-back", 
            false,
            closure_local!(@watch self as win => move |_clone_repository_page: BagitCloneRepositoryPage| {
                win.imp().stack.set_visible_child_name("main page");
            })
        );

        self.imp().clone_repository_page.connect_closure(
            "select-location", 
            false,
            closure_local!(@watch self as win => move |_clone_repository_page: BagitCloneRepositoryPage| {
                let ctx: MainContext = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak win as win2 => async move {
                    let dialog = gtk::FileDialog::builder()
                        .accept_label(gettext("_Add"))
                        .modal(true)
                        .title(gettext("_Select location"))
                        .build();

                    if let Ok(res) = dialog.select_folder_future(Some(&win2)).await {
                        win2.imp().clone_repository_page.imp().location_row.set_text(
                            res.path().unwrap_or(PathBuf::new()).to_str().unwrap()
                        );
                    }
                }));
            })
        );

        self.imp().clone_repository_page.connect_closure(
            "select-private-key", 
            false,
            closure_local!(@watch self as win => move |_clone_repository_page: BagitCloneRepositoryPage| {
                let ctx: MainContext = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak win as win2 => async move {
                    let dialog = gtk::FileDialog::builder()
                        .accept_label(gettext("_Add"))
                        .modal(true)
                        .title(gettext("_Select Private key path"))
                        .build();

                    if let Ok(res) = dialog.open_future(Some(&win2)).await {
                        win2.imp().clone_repository_page.imp().private_key_path.set_text(
                            res.path().unwrap_or(PathBuf::new()).to_str().unwrap()
                        );
                    }
                }));
            })
        );

        self.imp().clone_repository_page.connect_closure(
            "unique-name",
            false,
            closure_local!(@watch self as win => move |
                _clone_repository_page: BagitCloneRepositoryPage,
                image: gtk::Image,
                profile_name: &str,
                profile_id: &str
                | {
                    let same_profile_name_number = win.imp().app_database.get_number_of_git_profiles_with_name(
                        &profile_name,
                        &profile_id
                    );
                    image.set_visible(same_profile_name_number != 0);
            }),
        );

        self.imp().clone_repository_page.connect_closure(
            "clone-repository",
            false,
            closure_local!(@watch self as win => move |
                _clone_repository_page: BagitCloneRepositoryPage,
                url: &str,
                location: &str
                | {
                win.clone_repository(url, location);
            }),
        );

        self.imp().clone_repository_page.connect_closure(
            "clone-repository-and-add-profile",
            false,
            closure_local!(@watch self as win => move |
                _clone_repository_page: BagitCloneRepositoryPage,
                url: &str,
                location: &str,
                profile_name: &str,
                email: &str,
                username: &str,
                password: &str,
                private_key_path: &str
                | {
                let new_profile = BagitGitProfile::new(
                    Uuid::new_v4(),
                    profile_name.to_string(),
                    email.to_string(),
                    username.to_string(),
                    password.to_string(),
                    private_key_path.to_string(),
                );
                win.imp().app_database.add_git_profile(new_profile);
                win.clone_repository(url, location);
            }),
        );
    }

    fn repository_page_signals(&self) {
        self.imp().repository_page.connect_closure(
            "go-home",
            false,
            closure_local!(@watch self as win => move |_repository_page: BagitRepositoryPage| {
                win.imp().repositories_window.imp().all_repositories.unselect_all();
                win.imp().repositories_window.imp().recent_repositories.unselect_all();
                win.imp().stack.set_visible_child_name("main page");
            }),
        );
    }

    pub fn show_error_dialog(&self, error_message: &str) {
        let error_message = format!(
            "{}\n\n{}: {}",
            gettext("_An error has occured"),
            gettext("_Error message"),
            error_message
        );
        let ctx: MainContext = glib::MainContext::default();
        ctx.spawn_local(clone!(@weak self as win => async move {
            let alert_dialog = gtk::AlertDialog::builder()
            .modal(true)
            .message(error_message)
            .build();

            alert_dialog.show(Some(&win));
        }));
    }

    /**
     * Used to clone a repository.
     */
    pub fn clone_repository(&self, url: &str, location: &str) {
        let mut new_folder_name = self.get_folder_name_from_os(url);

        let replaced_text = &new_folder_name.replace(".git", "");
        new_folder_name = replaced_text.trim().to_owned();

        let new_folder_path = format!("{}/{}", &location, new_folder_name);

        let new_folder = fs::create_dir(&new_folder_path);

        match new_folder {
            Ok(_) => {
                let callback = if self.imp().clone_repository_page.is_using_https(&url) {
                    self.https_callback()
                } else {
                    self.ssh_callback()
                };

                let mut fo = git2::FetchOptions::new();
                fo.remote_callbacks(callback);

                let mut builder = git2::build::RepoBuilder::new();
                builder.fetch_options(fo);

                let repository = builder.clone(&url.trim(), Path::new(&new_folder_path));
                match repository {
                    Ok(_) => {
                        self.add_list_row(&new_folder_name, &new_folder_path);

                        let profile_id: Option<Uuid> = if self
                            .imp()
                            .clone_repository_page
                            .imp()
                            .git_profiles
                            .title()
                            .to_string()
                            == gettext("_No profile")
                        {
                            None
                        } else if self
                            .imp()
                            .clone_repository_page
                            .imp()
                            .git_profiles
                            .title()
                            .to_string()
                            == gettext("_New profile")
                        {
                            let profile = self.imp().app_database.get_git_profile_from_name(
                                self.imp()
                                    .clone_repository_page
                                    .imp()
                                    .profile_name
                                    .text()
                                    .as_str(),
                            );
                            if profile.is_some() {
                                Some(profile.unwrap().profile_id)
                            } else {
                                None
                            }
                        } else {
                            let profile = self.imp().app_database.get_git_profile_from_name(
                                self.imp()
                                    .clone_repository_page
                                    .imp()
                                    .git_profiles
                                    .title()
                                    .as_str(),
                            );
                            if profile.is_some() {
                                Some(profile.unwrap().profile_id)
                            } else {
                                None
                            }
                        };

                        self.imp().app_database.add_repository(
                            &new_folder_name,
                            &new_folder_path,
                            profile_id,
                        );
                        self.imp().stack.set_visible_child_name("main page");
                    }
                    Err(e) => {
                        // We must make sure to delete the created folder !
                        let removed_directory = fs::remove_dir_all(&new_folder_path);
                        match removed_directory {
                            Ok(_) => self.show_error_dialog(&e.to_string()),
                            Err(error) => self.show_error_dialog(&error.to_string()),
                        }
                    }
                }
            }
            Err(e) => self.show_error_dialog(&e.to_string()),
        }
    }

    /**
     * Used to create callback for https clone.
     */
    pub fn https_callback(&self) -> RemoteCallbacks<'_> {
        let mut callback = RemoteCallbacks::new();

        callback.credentials(|_url, username, _allowed_type| {
            if self
                .imp()
                .clone_repository_page
                .imp()
                .git_profiles
                .title()
                .to_string()
                == gettext("_No profile")
            {
                return Cred::userpass_plaintext(
                    if username.is_some() {
                        username.unwrap()
                    } else {
                        ""
                    },
                    "",
                );
            }

            let profile_name = if self
                .imp()
                .clone_repository_page
                .imp()
                .git_profiles
                .title()
                .to_string()
                == gettext("_New profile")
            {
                // We will use the information of the new profile :
                self.imp().clone_repository_page.imp().profile_name.text()
            } else {
                // We will use the information the selected profile :
                self.imp().clone_repository_page.imp().git_profiles.title()
            };

            let profile = self
                .imp()
                .app_database
                .get_git_profile_from_name(profile_name.as_str());
            if profile.is_some() {
                let found_profile = profile.unwrap();
                Cred::userpass_plaintext(&found_profile.username, &found_profile.password)
            } else {
                // If nothing is found, we return a default credential :
                Cred::userpass_plaintext(
                    if username.is_some() {
                        username.unwrap()
                    } else {
                        ""
                    },
                    "",
                )
            }
        });

        return callback;
    }

    /**
     * Used to create callback for ssh clone.
     */
    pub fn ssh_callback(&self) -> RemoteCallbacks<'_> {
        let mut callback = RemoteCallbacks::new();

        callback.credentials(|_url, username, _allowed_type| {
            let passphrase_to_use = self.imp().clone_repository_page.imp().passphrase.text();

            if self
                .imp()
                .clone_repository_page
                .imp()
                .git_profiles
                .title()
                .to_string()
                == gettext("_No profile")
            {
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
            }

            let profile_name = if self
                .imp()
                .clone_repository_page
                .imp()
                .git_profiles
                .title()
                .to_string()
                == gettext("_New profile")
            {
                // We will use the information of the new profile :
                self.imp().clone_repository_page.imp().profile_name.text()
            } else {
                // We will use the information of the selected profile :
                self.imp().clone_repository_page.imp().git_profiles.title()
            };

            let profile = self
                .imp()
                .app_database
                .get_git_profile_from_name(profile_name.as_str());
            if profile.is_some() {
                let found_profile = profile.unwrap();
                Cred::ssh_key(
                    if username.is_some() {
                        username.unwrap()
                    } else {
                        ""
                    },
                    None,
                    Path::new(&found_profile.private_key_path),
                    if passphrase_to_use.is_empty() {
                        None
                    } else {
                        Some(&passphrase_to_use)
                    },
                )
            } else {
                // If nothing is found, we return a default credential :
                Cred::ssh_key(
                    if username.is_some() {
                        username.unwrap()
                    } else {
                        ""
                    },
                    None,
                    Path::new(""),
                    None,
                )
            }
        });

        return callback;
    }

    /**
     * Used to get the folder name of a path from OS information.
     */
    pub fn get_folder_name_from_os(&self, path: &str) -> String {
        let os = env::consts::OS;

        // The path format changes depending on the OS.
        let folder_name = match os {
            "linux" | "macOS" | "freebsd" | "dragonfly" | "netbsd" | "openbsd" | "solaris" => {
                path.split("/").last().unwrap().to_string()
            }
            "windows" => path.split("\\").last().unwrap().to_string(),
            _ => "".to_string(),
        };
        return folder_name;
    }
}
