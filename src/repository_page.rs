/* repository_page.rs
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

use crate::models::bagit_git_profile::BagitGitProfile;
use crate::utils::repository_utils::RepositoryUtils;
use crate::utils::selected_repository::SelectedRepository;
use crate::widgets::repository::commits_sidebar::BagitCommitsSideBar;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::glib::subclass::Signal;
use gtk::glib::{clone, closure_local, MainContext, Priority};
use gtk::prelude::ObjectExt;
use gtk::subclass::widget::CompositeTemplateInitializingExt;
use gtk::template_callbacks;
use gtk::{glib, prelude::*, CompositeTemplate};
use itertools::Itertools;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::thread;
use uuid::Uuid;

use crate::utils::profile_mode::ProfileMode;
use crate::utils::{action_type::ActionType, clone_mode::CloneMode, db::AppDatabase};
use crate::widgets::repository::commit_view::BagitCommitView;

mod imp {

    use std::cell::Cell;

    use super::*;

    // Object holding the state
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/repository-page.ui")]
    pub struct BagitRepositoryPage {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub toggle_pane_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub flap: TemplateChild<adw::Flap>,
        #[template_child]
        pub sidebar: TemplateChild<BagitCommitsSideBar>,
        #[template_child]
        pub main_view_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub commit_view: TemplateChild<BagitCommitView>,
        #[template_child]
        pub pull_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub push_button: TemplateChild<gtk::Button>,

        pub app_database: AppDatabase,

        pub selected_repository: RefCell<SelectedRepository>,

        pub is_doing_git_action: Cell<bool>,
    }

    #[template_callbacks]
    impl BagitRepositoryPage {
        #[template_callback]
        fn go_home(&self, _button: gtk::Button) {
            self.obj().emit_by_name::<()>("go-home", &[]);
        }

        #[template_callback]
        fn pull(&self, _button: gtk::Button) {
            self.obj().do_git_action_with_auth_check(ActionType::PULL);
        }

        #[template_callback]
        fn push(&self, _button: gtk::Button) {
            self.obj().do_git_action_with_auth_check(ActionType::PUSH);
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitRepositoryPage {
        const NAME: &'static str = "BagitRepositoryPage";
        type Type = super::BagitRepositoryPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitRepositoryPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("go-home").build(),
                    Signal::builder("error")
                        .param_types([str::static_type()])
                        .build(),
                    Signal::builder("commit-files-with-signing-key")
                        .param_types([
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            bool::static_type(),
                        ])
                        .build(),
                    Signal::builder("missing-ssh-information")
                        .param_types([
                            str::static_type(),
                            str::static_type(),
                            ActionType::static_type(),
                        ])
                        .build(),
                    Signal::builder("missing-https-information")
                        .param_types([
                            str::static_type(),
                            str::static_type(),
                            ActionType::static_type(),
                        ])
                        .build(),
                    Signal::builder("ssh-passphrase-dialog")
                        .param_types([
                            str::static_type(),
                            str::static_type(),
                            ActionType::static_type(),
                        ])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_sidebar_signals();
            self.obj().connect_commit_view_signals();

            self.is_doing_git_action.set(false);
        }
    }
    impl WidgetImpl for BagitRepositoryPage {}
    impl BoxImpl for BagitRepositoryPage {}
}

glib::wrapper! {
    pub struct BagitRepositoryPage(ObjectSubclass<imp::BagitRepositoryPage>)
        @extends gtk::Widget,gtk::Window,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitRepositoryPage {
    /**
     * Used for connecting differents signals used by sidebar.
     */
    pub fn connect_sidebar_signals(&self) {
        self.imp().sidebar.connect_closure(
            "row-selected",
            false,
            closure_local!(@watch self as _win => move |
                _sidebar: BagitCommitsSideBar,
                index: i32
                | {
                    println!("Commit index: {}", index);
                }
            ),
        );

        self.imp().sidebar.connect_closure(
            "see-history",
            false,
            closure_local!(@watch self as _win => move |_sidebar: BagitCommitsSideBar| {
            }),
        );

        self.imp().sidebar.connect_closure(
            "update-changed-files",
            false,
            closure_local!(@watch self as win => move |
                _sidebar: BagitCommitsSideBar
                | {
                    win.update_changed_files();
                }
            ),
        );
        self.imp().sidebar.connect_closure(
            "show-commit-view",
            false,
            closure_local!(@watch self as win => move |
                _sidebar: BagitCommitsSideBar
                | {
                    win.imp().main_view_stack.set_visible_child_name("commit view");
                    win.imp().commit_view.update_commit_view(win.imp().sidebar.imp().changed_files.borrow().get_number_of_selected_files());
                    win.imp().flap.set_reveal_flap(false);
                }
            ),
        );
        self.imp().sidebar.connect_closure(
            "update-file-information-label",
            false,
            closure_local!(@watch self as win => move |
                _sidebar: BagitCommitsSideBar,
                total: i32
                | {
                    win.imp().commit_view.update_commit_view(total);
                }
            ),
        );
        self.imp().sidebar.connect_closure(
            "set-push-button-sensitive",
            false,
            closure_local!(@watch self as win => move |
                _sidebar: BagitCommitsSideBar,
                sensitive_value: bool
                | {
                    win.imp().push_button.set_sensitive(sensitive_value && !win.imp().is_doing_git_action.get());
                    win.imp().pull_button.set_sensitive(!sensitive_value && !win.imp().is_doing_git_action.get());
                }
            ),
        );
    }

    /// Used to connect signals send by the commit view.
    pub fn connect_commit_view_signals(&self) {
        self.imp().commit_view.connect_closure(
            "select-profile",
            false,
            closure_local!(@watch self as win => move |
                commit_view: BagitCommitView,
                profile_name: &str
                | {
                    let selected_profile = win.imp().app_database.get_git_profile_from_name(profile_name);
                    if selected_profile.is_some() {
                        let profile = selected_profile.unwrap();
                        let _ = match &win.imp().selected_repository.borrow().git_repository {
                            Some(repo) => RepositoryUtils::override_git_config(&repo, &profile),
                            None => Ok({}),
                        };
                        commit_view.set_and_show_selected_profile(profile.clone());
                        //...and we specify the new default profile used with the openned repository:
                        win.imp().app_database.change_git_profile_of_repository(
                            win.imp().selected_repository.borrow().user_repository.repository_id,
                            Some(profile.profile_id)
                        );
                        commit_view.update_commit_view(
                            win.imp().sidebar.imp().changed_files.borrow().get_number_of_selected_files()
                        );
                    }
                }
            ),
        );
        self.imp().commit_view.connect_closure(
            "remove-profile",
            false,
            closure_local!(@watch self as win => move |
                commit_view: BagitCommitView
                | {
                    win.imp().app_database.change_git_profile_of_repository(
                        win.imp().selected_repository.borrow().user_repository.repository_id,
                        None
                    );
                    let _ = match &win.imp().selected_repository.borrow().git_repository {
                        Some(repo) => RepositoryUtils::reset_git_config(&repo),
                        None => Ok({}),
                    };
                    commit_view.update_commit_view(
                        win.imp().sidebar.imp().changed_files.borrow().get_number_of_selected_files()
                    );
                }
            ),
        );
        self.imp().commit_view.connect_closure(
            "commit-files",
            false,
            closure_local!(@watch self as win => move |
                _commit_view: BagitCommitView,
                author: &str,
                author_email: &str,
                message: &str,
                signing_key: &str,
                description: &str,
                need_to_save_profile: bool
                | {
                    if signing_key.trim().is_empty() {
                        win.commit_files_and_update_ui(
                            author,
                            author_email,
                            message,
                            signing_key,
                            "",
                            description,
                            need_to_save_profile
                        );
                    } else {
                        win.imp().obj().emit_by_name::<()>(
                            "commit-files-with-signing-key",
                            &[
                                &author,
                                &author_email,
                                &message,
                                &signing_key,
                                &description,
                                &need_to_save_profile,
                            ],
                        );
                    }
                }
            ),
        );
        self.imp().commit_view.connect_closure(
            "toggle-commit-button",
            false,
            closure_local!(@watch self as win => move |
                commit_view: BagitCommitView,
                | {
                    commit_view.update_commit_view(
                        win.imp().sidebar.imp().changed_files.borrow().get_number_of_selected_files()
                    );
                }
            ),
        );
        self.imp().commit_view.connect_closure(
            "update-git-config",
            false,
            closure_local!(@watch self as win => move |
                _commit_view: BagitCommitView,
                | {
                    let profile_mode = win.imp().commit_view.imp().profile_mode.borrow().get_profile_mode();
                    match profile_mode {
                        ProfileMode::SelectedProfile(profile) => {
                            match &win.imp().selected_repository.borrow().git_repository {
                                Some(repository) => RepositoryUtils::override_git_config(&repository, &profile).unwrap(),
                                None => {},
                            };
                        },
                        _ => {}
                    };
                }
            ),
        );
    }

    /// Used to initialize the repository page with a selected repository.
    pub fn init_repository_page(&self, repository: SelectedRepository) {
        self.imp()
            .main_view_stack
            .set_visible_child_name("hello world");

        self.imp().sidebar.init_commits_sidebar();
        self.imp().commit_view.init_commit_view();

        if repository.user_repository.git_profile_id.is_some() {
            let selected_profile = self
                .imp()
                .app_database
                .get_git_profile_from_id(repository.user_repository.git_profile_id.unwrap());
            match selected_profile {
                Some(profile) => {
                    self.imp()
                        .commit_view
                        .set_and_show_selected_profile(profile);
                }
                None => self.imp().commit_view.show_no_selected_profile(),
            }
        } else {
            self.imp().commit_view.show_no_selected_profile();
        }

        self.imp().selected_repository.replace(repository);

        self.update_commits_sidebar();
    }

    /// Updates the changed files and commit history.
    pub fn update_commits_sidebar(&self) {
        let selected_repository = self.imp().selected_repository.take();

        let selected_repository_path = selected_repository.user_repository.path.clone();

        self.imp().selected_repository.replace(selected_repository);

        self.imp()
            .sidebar
            .refresh_commit_list_if_needed(selected_repository_path);

        self.update_changed_files();
    }

    /**
     * Used to update changed files list.
     */
    fn update_changed_files(&self) {
        let borrowed_repo = self.imp().selected_repository.borrow();
        if borrowed_repo.git_repository.is_some() {
            let repo = borrowed_repo.git_repository.as_ref().unwrap();
            match repo.statuses(None) {
                Ok(statuses) => {
                    self.imp().sidebar.clear_changed_ui_files_list();
                    let hash_map = self.imp().sidebar.build_hash_map(statuses);

                    for key in hash_map.keys().sorted() {
                        let value = &hash_map[key];
                        if key != "" {
                            let borrowed_changed_folders =
                                self.imp().sidebar.imp().changed_files.borrow();
                            let folder = borrowed_changed_folders
                                .get_changed_folder_from_list(&key)
                                .unwrap();
                            self.imp()
                                .sidebar
                                .generate_folder(folder, (&value).to_vec());
                        }
                    }
                    if hash_map.contains_key("") {
                        for file in &hash_map[""] {
                            let new_row =
                                self.imp().sidebar.generate_changed_file(file, 4, 8, None);
                            self.imp().sidebar.imp().menu.append(&new_row.0);
                        }
                    }
                }
                Err(_) => {}
            };
        }
    }

    /// Used to commit files and update UI.
    pub fn commit_files_and_update_ui(
        &self,
        author: &str,
        author_email: &str,
        message: &str,
        signing_key: &str,
        passphrase: &str,
        description: &str,
        need_to_save_profile: bool,
    ) {
        let borrowed_repo = self.imp().selected_repository.take();
        if borrowed_repo.git_repository.is_some() {
            let git_repository = borrowed_repo.git_repository.as_ref().unwrap();
            let selected_files = self
                .imp()
                .sidebar
                .imp()
                .changed_files
                .borrow()
                .get_selected_files();

            // We save the profile if we need to :
            if need_to_save_profile {
                let new_profile_id = Uuid::new_v4();

                // We make sure that the profile name is unique :
                let same_profile_name_number = self
                    .imp()
                    .app_database
                    .get_number_of_git_profiles_with_name(&author, &new_profile_id.to_string());
                let final_profil_name: String = if same_profile_name_number != 0 {
                    let new_name = format!("{} ({})", author, same_profile_name_number);
                    new_name
                } else {
                    author.to_string()
                };

                let new_profile = BagitGitProfile::new(
                    new_profile_id.clone(),
                    final_profil_name,
                    author_email.to_string(),
                    author.to_string(),
                    String::from(""),
                    String::from(""),
                    signing_key.to_string(),
                );
                self.imp().app_database.add_git_profile(&new_profile);

                // We set the new profile to the repository:
                self.imp().app_database.change_git_profile_of_repository(
                    borrowed_repo.user_repository.repository_id,
                    Some(new_profile_id),
                );
                self.imp()
                    .commit_view
                    .imp()
                    .profile_mode
                    .replace(ProfileMode::SelectedProfile(new_profile));

                // We update the view:
                self.imp().commit_view.update_git_profiles_list();
            }
            match RepositoryUtils::commit_files(
                git_repository,
                selected_files,
                message,
                description,
                author,
                author_email,
                signing_key,
                passphrase,
            ) {
                Ok(_) => {
                    let toast = adw::Toast::new(&gettext("_Commit created successfully"));
                    self.imp().toast_overlay.add_toast(toast);
                    // We remove the last commit message:
                    self.imp().commit_view.imp().message_row.set_text("");
                    self.imp().commit_view.imp().description_row.set_text("");
                    self.imp().selected_repository.replace(borrowed_repo);
                    self.update_commits_sidebar();
                    self.imp().commit_view.update_commit_view(0);
                }
                Err(error) => {
                    self.imp().selected_repository.replace(borrowed_repo);
                    self.emit_by_name("commit-error", &[&error.to_string()])
                }
            }
        }
    }
    /// Used to block git action buttons (if we are doing an async action, we need to block them).
    fn block_git_action_buttons(&self) {
        self.imp().is_doing_git_action.set(true);

        self.imp().pull_button.set_sensitive(false);
        self.imp().push_button.set_sensitive(false);
    }

    /// Check if we can pull (if we have no changed files).
    fn check_if_can_pull(&self) -> bool {
        let changed_files_number = self
            .imp()
            .sidebar
            .imp()
            .changed_files
            .borrow()
            .get_number_of_changed_files();

        return changed_files_number == 0;
    }

    /// Used to define wich git action we need to do:
    pub fn do_git_action_with_information(
        &self,
        username: String,
        password: String,
        private_key_path: String,
        passphrase: String,
        action_type: ActionType,
    ) {
        match action_type {
            ActionType::PUSH => {
                self.push_and_update_ui(username, password, private_key_path, passphrase)
            }
            ActionType::PULL => {
                self.pull_and_update_ui(username, password, private_key_path, passphrase)
            }
        };
    }

    /// Used to do a git action that need authentification.
    fn do_git_action_with_auth_check(&self, action_type: ActionType) {
        let selected_repository = self.imp().selected_repository.take();
        let profile_mode = self.imp().commit_view.imp().profile_mode.take();

        self.imp().selected_repository.replace(
            SelectedRepository::try_fetching_selected_repository(
                &selected_repository.user_repository,
            )
            .unwrap(),
        );

        self.imp()
            .commit_view
            .imp()
            .profile_mode
            .replace(profile_mode.clone());

        // If we want to pull, we need to check that we don't have changed files:
        if action_type == ActionType::PULL && !self.check_if_can_pull() {
            let toast = adw::Toast::new(&gettext("_Changed files when pull"));
            self.imp().toast_overlay.add_toast(toast);
            return;
        }

        match &selected_repository.git_repository {
            Some(repository) => {
                match RepositoryUtils::get_clone_mode_of_repository(&repository) {
                    Ok(clone_mode) => match profile_mode {
                        ProfileMode::SelectedProfile(profile) => {
                            if !profile.does_profile_has_information_for_actions(&clone_mode) {
                                match clone_mode {
                                    CloneMode::SSH => self.emit_by_name::<()>(
                                        "missing-ssh-information",
                                        &[
                                            &profile.username,
                                            &profile.private_key_path,
                                            &action_type,
                                        ],
                                    ),
                                    CloneMode::HTTPS => self.emit_by_name::<()>(
                                        "missing-https-information",
                                        &[&profile.username, &profile.password, &action_type],
                                    ),
                                };
                            } else {
                                match clone_mode {
                                    CloneMode::SSH => self.emit_by_name::<()>(
                                        "ssh-passphrase-dialog",
                                        &[
                                            &profile.username,
                                            &profile.private_key_path,
                                            &action_type,
                                        ],
                                    ),
                                    CloneMode::HTTPS => self.do_git_action_with_information(
                                        profile.username,
                                        profile.password,
                                        profile.private_key_path,
                                        "".to_string(),
                                        action_type,
                                    ),
                                };
                            }
                        }
                        _ => {
                            match clone_mode {
                                CloneMode::SSH => self.emit_by_name::<()>(
                                    "missing-ssh-information",
                                    &[
                                        &self.imp().commit_view.imp().author_row.text().trim(),
                                        &"",
                                        &action_type,
                                    ],
                                ),
                                CloneMode::HTTPS => self.emit_by_name::<()>(
                                    "missing-https-information",
                                    &[
                                        &self.imp().commit_view.imp().author_row.text().trim(),
                                        &"",
                                        &action_type,
                                    ],
                                ),
                            };
                        }
                    },
                    Err(error) => self.emit_by_name::<()>("error", &[&error.to_string()]),
                };
            }
            None => {}
        }
    }

    /// Used to push and update ui.
    pub fn push_and_update_ui(
        &self,
        username: String,
        password: String,
        private_key_path: String,
        passphrase: String,
    ) {
        let selected_repository = self.imp().selected_repository.take();
        self.imp().selected_repository.replace(
            SelectedRepository::try_fetching_selected_repository(
                &selected_repository.user_repository,
            )
            .unwrap(),
        );

        let (error_sender, error_receiver) = MainContext::channel::<String>(Priority::default());
        let (result_sender, result_receiver) = MainContext::channel::<()>(Priority::default());

        self.block_git_action_buttons();

        thread::spawn(move || {
            let error_sender = error_sender.clone();
            let result_sender = result_sender.clone();

            match RepositoryUtils::push(
                &selected_repository.git_repository.as_ref().unwrap(),
                username,
                password,
                private_key_path,
                passphrase,
            ) {
                Ok(_) => result_sender
                    .send(())
                    .expect("Could not send result through channel"),
                Err(error) => error_sender
                    .send(error.to_string())
                    .expect("Could not send error through channel"),
            };
        });

        error_receiver.attach(
            None,
            clone!(@weak self as win => @default-return Continue(false),
                        move |error| {
                            win.emit_by_name::<()>("error", &[&error.to_string()]);
                            win.imp().is_doing_git_action.set(false);
                            win.update_commits_sidebar();
                            Continue(true)
                        }
            ),
        );

        result_receiver.attach(
            None,
            clone!(
                @weak self as win => @default-return Continue(false),
                        move |_| {
                            let toast = adw::Toast::new(&gettext("_Commits pushed"));
                            win.imp().toast_overlay.add_toast(toast);
                            win.imp().is_doing_git_action.set(false);
                            win.update_commits_sidebar();
                            Continue(true)
                        }
            ),
        );
    }

    /// Used to push and update ui.
    pub fn pull_and_update_ui(
        &self,
        username: String,
        password: String,
        private_key_path: String,
        passphrase: String,
    ) {
        let selected_repository = self.imp().selected_repository.take();
        self.imp().selected_repository.replace(
            SelectedRepository::try_fetching_selected_repository(
                &selected_repository.user_repository,
            )
            .unwrap(),
        );

        let (error_sender, error_receiver) = MainContext::channel::<String>(Priority::default());
        let (result_sender, result_receiver) = MainContext::channel::<()>(Priority::default());

        self.block_git_action_buttons();

        thread::spawn(move || {
            let error_sender = error_sender.clone();
            let result_sender = result_sender.clone();

            match RepositoryUtils::pull(
                &selected_repository.git_repository.as_ref().unwrap(),
                username,
                password,
                private_key_path,
                passphrase,
            ) {
                Ok(_) => result_sender
                    .send(())
                    .expect("Could not send result through channel"),
                Err(error) => error_sender
                    .send(error.to_string())
                    .expect("Could not send error through channel"),
            };
        });

        error_receiver.attach(
            None,
            clone!(@weak self as win => @default-return Continue(false),
                        move |error| {
                            win.emit_by_name::<()>("error", &[&error.to_string()]);
                            win.imp().is_doing_git_action.set(false);
                            win.update_commits_sidebar();
                            Continue(true)
                        }
            ),
        );

        result_receiver.attach(
            None,
            clone!(
                @weak self as win => @default-return Continue(false),
                        move |_| {
                            let toast = adw::Toast::new(&gettext("_Remote branch pulled"));
                            win.imp().toast_overlay.add_toast(toast);
                            win.imp().is_doing_git_action.set(false);
                            win.update_commits_sidebar();
                            Continue(true)
                        }
            ),
        );
    }
}
