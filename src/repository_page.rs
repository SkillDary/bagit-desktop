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

use std::path::Path;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;

use crate::models::bagit_git_profile::BagitGitProfile;
use crate::utils::action_type::ActionType;
use crate::utils::changed_file::ChangedFile;
use crate::utils::clone_mode::CloneMode;
use crate::utils::fetch_result::FetchResult;
use crate::utils::git::fetch_checked_out_branch;
use crate::utils::profile_mode::ProfileMode;
use crate::utils::repository_utils::RepositoryUtils;
use crate::utils::selected_repository::SelectedRepository;
use crate::widgets::repository::commit_view::BagitCommitView;
use crate::widgets::repository::commits_sidebar::BagitCommitsSideBar;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use git2::Repository;
use gtk::glib::subclass::Signal;
use gtk::glib::{clone, closure_local, MainContext, Priority};
use gtk::{glib, prelude::*, CompositeTemplate};
use itertools::Itertools;
use notify::{RecursiveMode, Watcher};
use uuid::Uuid;

mod imp {

    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
        sync::mpsc,
    };

    use adw::SplitButton;
    use gtk::{template_callbacks, Label, Spinner};
    use once_cell::sync::Lazy;

    use crate::{
        utils::{action_type::ActionType, db::AppDatabase},
        widgets::repository::commit_view::BagitCommitView,
    };

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
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub flap: TemplateChild<adw::Flap>,
        #[template_child]
        pub sidebar: TemplateChild<BagitCommitsSideBar>,
        #[template_child]
        pub main_view_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub commit_view: TemplateChild<BagitCommitView>,
        #[template_child]
        pub branch_button_content: TemplateChild<adw::ButtonContent>,
        #[template_child]
        pub repository_name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub git_action_button: TemplateChild<SplitButton>,
        #[template_child]
        pub fetch_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub pull_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub push_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub git_action_label: TemplateChild<Label>,
        #[template_child]
        pub git_action_spinner: TemplateChild<Spinner>,
        #[template_child]
        pub push_indication_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub git_action_push_number: TemplateChild<Label>,
        #[template_child]
        pub pull_indication_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub git_action_pull_number: TemplateChild<Label>,

        pub app_database: RefCell<AppDatabase>,

        pub selected_repository: RefCell<SelectedRepository>,

        pub current_git_action: RefCell<ActionType>,
        pub is_doing_git_action: Cell<bool>,

        pub ssh_passphrases: RefCell<HashMap<String, String>>,

        pub directory_watcher_thread_mpsc_sender: RefCell<Option<mpsc::Sender<()>>>,
    }

    #[template_callbacks]
    impl BagitRepositoryPage {
        #[template_callback]
        fn go_home(&self, _button: gtk::Button) {
            self.obj().kill_existing_file_watcher();

            self.obj().emit_by_name::<()>("go-home", &[]);
        }

        #[template_callback]
        fn git_action(&self, _split_button: SplitButton) {
            let current_git_action = self.current_git_action.take();

            self.current_git_action.replace(current_git_action);

            match current_git_action {
                ActionType::PUSH => self.obj().do_git_action_with_auth_check(ActionType::PUSH),
                _ => self
                    .obj()
                    .try_do_git_action_without_auth_check(current_git_action),
            }
        }

        #[template_callback]
        fn fetch(&self, _button: gtk::Button) {
            self.obj()
                .update_git_action_button_action(ActionType::FETCH);
            self.obj()
                .try_do_git_action_without_auth_check(ActionType::FETCH);
        }

        #[template_callback]
        fn pull(&self, _button: gtk::Button) {
            self.obj().update_git_action_button_action(ActionType::PULL);
            self.obj()
                .try_do_git_action_without_auth_check(ActionType::PULL);
        }

        #[template_callback]
        fn push(&self, _button: gtk::Button) {
            self.obj().update_git_action_button_action(ActionType::PUSH);
            self.obj().do_git_action_with_auth_check(ActionType::PUSH);
        }

        #[template_callback]
        fn branch_button_action(&self, _button: gtk::Button) {
            let repository = self.selected_repository.borrow();
            self.obj()
                .emit_by_name::<()>("select-branch", &[&repository.user_repository.path]);
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
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_sidebar_signals();
            self.obj().connect_commit_view_signals();

            let mut app_database = self.app_database.take();

            app_database.create_connection();

            self.app_database.replace(app_database);

            self.is_doing_git_action.set(false);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("go-home").build(),
                    Signal::builder("error")
                        .param_types([str::static_type()])
                        .build(),
                    Signal::builder("select-branch")
                        .param_types([str::static_type()])
                        .build(),
                    Signal::builder("discard-dialog")
                        .param_types([bool::static_type(), str::static_type()])
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
            "update-git-action-button",
            false,
            closure_local!(@watch self as win => move |
                _sidebar: BagitCommitsSideBar,
                local_commits: i32
                | {
                    win.update_push_indication_box(local_commits.into());
                    if local_commits != 0 {
                        win.update_git_action_button_action(ActionType::PUSH);
                    } else {
                        win.update_git_action_button_action(ActionType::FETCH);
                    }
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

                    if win.imp().flap.is_folded() {
                        win.imp().flap.set_reveal_flap(false);
                    }
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
            "discard-file",
            false,
            closure_local!(@watch self as win => move |
                _sidebar: BagitCommitsSideBar,
                file_path: &str
                | {
                    win.emit_by_name::<()>("discard-dialog", &[&false, &file_path]);
                }
            ),
        );
        self.imp().sidebar.connect_closure(
            "discard-folder",
            false,
            closure_local!(@watch self as win => move |
                _sidebar: BagitCommitsSideBar,
                folder_path: &str
                | {
                    win.emit_by_name::<()>("discard-dialog", &[&true, &folder_path]);
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
                    let selected_profile;

                    let app_database = win.imp().app_database.take();

                    match app_database.get_git_profile_from_name(profile_name) {
                        Ok(profile) => selected_profile = profile,
                        Err(error) => {
                            // TODO: Show error (maybe with a toast).

                            tracing::warn!("Could not get Git profile from name: {}", error);

                            return;
                        },
                    }

                    if selected_profile.is_some() {
                        let profile = selected_profile.unwrap();
                        let _ = match &win.imp().selected_repository.borrow().git_repository {
                            Some(repo) => RepositoryUtils::override_git_config(&repo, &profile),
                            None => Ok({}),
                        };
                        commit_view.set_and_show_selected_profile(profile.clone());

                        //...and we specify the new default profile used with the openned repository:
                        if let Err(error) = app_database.change_git_profile_of_repository(
                            win.imp().selected_repository.borrow().user_repository.repository_id,
                            Some(profile.profile_id)
                        ) {
                            tracing::warn!("Could not change Git profile of repository: {}", error);

                            let toast = adw::Toast::new(&gettext("_Could not change Git profile"));
                            win.imp().toast_overlay.add_toast(toast);
                        }

                        commit_view.update_commit_view(
                            win.imp().sidebar.imp().changed_files.borrow().get_number_of_selected_files()
                        );
                    }

                    win.imp().app_database.replace(app_database);
                }
            ),
        );
        self.imp().commit_view.connect_closure(
            "remove-profile",
            false,
            closure_local!(@watch self as win => move |
                commit_view: BagitCommitView
                | {
                    let app_database = win.imp().app_database.take();

                    if let Err(error) = app_database.change_git_profile_of_repository(
                        win.imp().selected_repository.borrow().user_repository.repository_id,
                        None
                    ) {
                        tracing::warn!("Could not change Git profile of repository: {}", error);

                        let toast = adw::Toast::new(&gettext("_Could not change Git profile"));
                        win.imp().toast_overlay.add_toast(toast);
                    }

                    win.imp().app_database.replace(app_database);

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

    /// Kills the existing file watcher if there is one.
    fn kill_existing_file_watcher(&self) {
        let sender = self.imp().directory_watcher_thread_mpsc_sender.take();

        // If there is a sender, then it means a thread already exist.
        if let Some(sender) = sender {
            // Kill the previous thread.
            if let Err(error) = sender.send(()) {
                tracing::error!("Could not kill existing file watcher: {}", error);
            }
        }

        self.imp()
            .directory_watcher_thread_mpsc_sender
            .replace(None);
    }

    /// Creates a file watcher on a separate thread.
    /// When a file change is detected, the UI is updated.
    /// Before creating another thread, the previous one, if it exists, is killed.
    fn create_file_watcher(&self, path: &Path) {
        let path = path.to_owned();

        self.kill_existing_file_watcher();

        let (mpsc_sender, mpsc_receiver) = mpsc::channel::<()>();

        let (glib_sender, glib_receiver) = MainContext::channel::<()>(Priority::default());

        thread::spawn(move || {
            let mut watcher = notify::recommended_watcher(move |res| match res {
                Ok(_event) => {
                    glib_sender
                        .send(())
                        .expect("Could not send through channel");
                }
                Err(error) => tracing::error!("Error on file watcher: {}", error),
            })
            .unwrap();

            // All files and directories at that path and below will be monitored for changes.
            // TODO: On some platforms, if the path is renamed or removed while being watched,
            // behaviour may be unexpected. If less surprising behaviour is wanted one may
            // non-recursively watch the parent directory as well and manage related events.
            watcher.watch(&path, RecursiveMode::Recursive).unwrap();

            loop {
                match mpsc_receiver.try_recv() {
                    Ok(_) | Err(TryRecvError::Disconnected) => {
                        break;
                    }
                    Err(TryRecvError::Empty) => {}
                }
            }
        });

        self.imp()
            .directory_watcher_thread_mpsc_sender
            .replace(Some(mpsc_sender));

        glib_receiver.attach(
            None,
            clone!(@weak self as win => @default-return Continue(false),
                        move |_event| {
                            win.update_changed_files();
                            Continue(true)
                        }
            ),
        );
    }

    /// Used to initialize the repository page with a selected repository.
    pub fn init_repository_page(&self, repository: SelectedRepository) {
        self.create_file_watcher(Path::new(&repository.user_repository.path));

        self.imp()
            .main_view_stack
            .set_visible_child_name("hello page");

        let status_page_title = format!(
            "{} {}.",
            gettext("_You are on"),
            repository.user_repository.name
        );

        self.imp().status_page.set_title(&status_page_title);

        self.init_git_action_button();
        self.imp().sidebar.init_commits_sidebar();
        self.imp()
            .sidebar
            .init_commit_list(repository.user_repository.path.clone());
        self.imp().commit_view.init_commit_view();

        self.imp()
            .repository_name_label
            .set_label(&repository.user_repository.name);

        if repository.user_repository.git_profile_id.is_some() {
            let selected_profile;

            let app_database = self.imp().app_database.take();

            match app_database
                .get_git_profile_from_id(repository.user_repository.git_profile_id.unwrap())
            {
                Ok(profile) => selected_profile = profile,
                Err(error) => {
                    // TODO: Show error (maybe with a toast).

                    tracing::warn!("Could not get Git profile from ID: {}", error);

                    return;
                }
            }

            self.imp().app_database.replace(app_database);

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
        self.update_branch_name();
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

    pub fn update_branch_name(&self) {
        let borrowed_repo = self.imp().selected_repository.borrow();
        match &borrowed_repo.git_repository {
            Some(repository) => match RepositoryUtils::get_current_branch_name(&repository) {
                Ok(branch_name) => self.imp().branch_button_content.set_label(&branch_name),
                Err(_) => {}
            },
            None => {}
        }
    }

    /// Initialize the git action button.
    pub fn init_git_action_button(&self) {
        self.update_push_indication_box(0);
        self.update_pull_indication_box(0);
        self.update_git_action_button_action(ActionType::FETCH);
    }

    /// Used to try to find the correct git button action.
    /// The default action is fetch, and the preffered one is push.
    pub fn try_to_find_correct_git_button_action(&self) {
        if self.imp().push_indication_box.is_visible() {
            self.update_git_action_button_action(ActionType::PUSH);
            return;
        }
        if self.imp().pull_indication_box.is_visible() {
            self.update_git_action_button_action(ActionType::PULL);
            return;
        }
        self.update_git_action_button_action(ActionType::FETCH);
    }

    /// Used to update the popover menu buttons visibility.
    fn update_popover_menu_buttons(&self, action_type: ActionType) {
        self.imp()
            .fetch_button
            .set_visible(!(action_type == ActionType::FETCH));
        self.imp()
            .pull_button
            .set_visible(!(action_type == ActionType::PULL));
        self.imp()
            .push_button
            .set_visible(!(action_type == ActionType::PUSH));
    }

    /// USed to update the git action button text and current action.
    fn update_git_action_button_action(&self, action_type: ActionType) {
        match action_type {
            ActionType::FETCH => {
                self.imp().current_git_action.replace(ActionType::FETCH);

                self.imp()
                    .git_action_label
                    .get()
                    .set_label(&gettext("_Fetch"));
            }
            ActionType::PUSH => {
                self.imp().current_git_action.replace(ActionType::PUSH);

                self.imp()
                    .git_action_label
                    .get()
                    .set_label(&gettext("_Push"));
            }
            ActionType::PULL => {
                self.imp().current_git_action.replace(ActionType::PULL);

                self.imp()
                    .git_action_label
                    .get()
                    .set_label(&gettext("_Pull"));
            }
        }
        self.update_popover_menu_buttons(action_type);
    }

    /// Used to update the push indication box
    fn update_push_indication_box(&self, total_commits_to_push: i64) {
        self.imp()
            .push_indication_box
            .set_visible(total_commits_to_push != 0);

        self.imp()
            .git_action_push_number
            .set_label(&total_commits_to_push.to_string());
    }

    /// Used to update the pull indication box
    fn update_pull_indication_box(&self, total_commits_to_pull: i64) {
        self.imp()
            .pull_indication_box
            .set_visible(total_commits_to_pull != 0);

        self.imp()
            .git_action_pull_number
            .set_label(&total_commits_to_pull.to_string());
    }

    /// Used to activate or desactivate the action button.
    pub fn toggle_git_action_button(&self, is_active: bool) {
        self.imp().git_action_button.set_sensitive(is_active);
        self.imp().git_action_spinner.set_visible(!is_active);
    }

    /// Updates the git action button.
    pub fn update_git_action_button(&self, fetch_result: FetchResult) {
        self.update_push_indication_box(fetch_result.total_commits_to_push);
        self.update_pull_indication_box(fetch_result.total_commits_to_pull);

        if fetch_result.total_commits_to_push == 0 && fetch_result.total_commits_to_pull == 0 {
            self.update_git_action_button_action(ActionType::FETCH);
            self.toggle_git_action_button(true);
            return;
        }

        if fetch_result.total_commits_to_push > 0 {
            self.update_git_action_button_action(ActionType::PUSH);
        } else {
            self.update_git_action_button_action(ActionType::PULL);
        }

        self.toggle_git_action_button(true);
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

                // We make sure that the profile name is unique:
                let same_profile_name_number;

                let app_database = self.imp().app_database.take();

                match app_database
                    .get_number_of_git_profiles_with_name(&author, &new_profile_id.to_string())
                {
                    Ok(number) => same_profile_name_number = number,
                    Err(error) => {
                        // TODO: Show error (maybe with a toast).

                        tracing::warn!("Could not get number of git profiles with name: {}", error);

                        return;
                    }
                }

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

                if let Err(error) = app_database.add_git_profile(&new_profile) {
                    tracing::warn!("Could not add Git profile: {}", error);

                    let toast = adw::Toast::new(&gettext("_Could not add Git profile"));
                    self.imp().toast_overlay.add_toast(toast);
                }

                // We set the new profile to the repository:
                if let Err(error) = app_database.change_git_profile_of_repository(
                    borrowed_repo.user_repository.repository_id,
                    Some(new_profile_id),
                ) {
                    tracing::warn!("Could not change Git profile of repository: {}", error);

                    let toast = adw::Toast::new(&gettext("_Could not change Git profile"));
                    self.imp().toast_overlay.add_toast(toast);
                }

                self.imp().app_database.replace(app_database);

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
                    self.emit_by_name("error", &[&error.to_string()])
                }
            }
        }
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
            ActionType::FETCH => self.fetch_repository_checked_out_branch_and_update_ui(
                username,
                password,
                private_key_path,
                passphrase,
            ),
            ActionType::PUSH => {
                self.push_and_update_ui(username, password, private_key_path, passphrase)
            }
            ActionType::PULL => {
                self.pull_and_update_ui(username, password, private_key_path, passphrase)
            }
        };
    }

    fn try_do_git_action_without_auth_check(&self, action_type: ActionType) {
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
            self.toggle_git_action_button(true);
            self.try_to_find_correct_git_button_action();
            return;
        }

        match action_type {
            ActionType::FETCH => return self.try_fetch_without_auth_and_update_ui(),
            ActionType::PULL => return self.try_pull_without_auth_and_update_ui(),
            _ => {}
        };
    }

    fn retrieve_saved_ssh_passphrase(&self, private_key_path: &str) -> Option<String> {
        let ssh_passphrases = self.imp().ssh_passphrases.take();

        let res = ssh_passphrases.get(private_key_path).clone();

        self.imp()
            .ssh_passphrases
            .replace(ssh_passphrases.to_owned());

        match res {
            Some(passphrase) => Some(passphrase.to_owned()),
            None => None,
        }
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
            self.toggle_git_action_button(true);
            self.try_to_find_correct_git_button_action();
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
                                // TODO: MARKER
                                match clone_mode {
                                    CloneMode::SSH => {
                                        match self.retrieve_saved_ssh_passphrase(
                                            &profile.private_key_path,
                                        ) {
                                            Some(passphrase) => self
                                                .do_git_action_with_information(
                                                    profile.username,
                                                    profile.password,
                                                    profile.private_key_path,
                                                    passphrase.to_owned(),
                                                    action_type,
                                                ),
                                            None => self.emit_by_name::<()>(
                                                "ssh-passphrase-dialog",
                                                &[
                                                    &profile.username,
                                                    &profile.private_key_path,
                                                    &action_type,
                                                ],
                                            ),
                                        }
                                    }
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

        self.toggle_git_action_button(false);

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

                            win.try_to_find_correct_git_button_action();
                            win.toggle_git_action_button(true);

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

                            win.update_push_indication_box(0);
                            win.try_to_find_correct_git_button_action();
                            win.toggle_git_action_button(true);

                            win.imp().sidebar.imp().first_commit_oid_of_commit_list.take();
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

        self.toggle_git_action_button(false);

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

                            win.try_to_find_correct_git_button_action();
                            win.toggle_git_action_button(true);

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

                            win.update_pull_indication_box(0);
                            win.try_to_find_correct_git_button_action();
                            win.toggle_git_action_button(true);

                            win.update_commits_sidebar();
                            Continue(true)
                        }
            ),
        );
    }

    /// Used to push and update ui.
    pub fn try_pull_without_auth_and_update_ui(&self) {
        let selected_repository = self.imp().selected_repository.take();
        self.imp().selected_repository.replace(
            SelectedRepository::try_fetching_selected_repository(
                &selected_repository.user_repository,
            )
            .unwrap(),
        );

        let (error_sender, error_receiver) =
            MainContext::channel::<git2::Error>(Priority::default());
        let (result_sender, result_receiver) = MainContext::channel::<()>(Priority::default());

        self.toggle_git_action_button(false);

        thread::spawn(move || {
            let error_sender = error_sender.clone();
            let result_sender = result_sender.clone();

            match RepositoryUtils::pull(
                &selected_repository.git_repository.as_ref().unwrap(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ) {
                Ok(_) => result_sender
                    .send(())
                    .expect("Could not send result through channel"),
                Err(error) => error_sender
                    .send(error)
                    .expect("Could not send error through channel"),
            };
        });

        error_receiver.attach(
            None,
            clone!(@weak self as win => @default-return Continue(false),
                        move |error| {
                            if (error.class() == git2::ErrorClass::Http)
                                || (error.class() == git2::ErrorClass::Ssh)
                            {
                                win.do_git_action_with_auth_check(ActionType::PULL);
                            } else {
                                win.emit_by_name::<()>("error", &[&error.to_string()]);

                                win.try_to_find_correct_git_button_action();
                                win.toggle_git_action_button(true);

                                win.update_commits_sidebar();
                            }
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

                            win.update_pull_indication_box(0);
                            win.try_to_find_correct_git_button_action();
                            win.toggle_git_action_button(true);

                            win.update_commits_sidebar();
                            Continue(true)
                        }
            ),
        );
    }

    /// Used to discard a file and update UI.
    pub fn discard_file_and_update_ui(&self, file_path: &str) {
        let selected_repository = self.imp().selected_repository.take();
        self.imp().selected_repository.replace(
            SelectedRepository::try_fetching_selected_repository(
                &selected_repository.user_repository,
            )
            .unwrap(),
        );
        match RepositoryUtils::discard_one_file(
            &selected_repository.git_repository.unwrap(),
            file_path,
        ) {
            Ok(_) => {
                self.update_commits_sidebar();
            }
            Err(error) => self.emit_by_name::<()>("error", &[&error.to_string()]),
        };
    }

    /// Used to change the current branch.
    pub fn checkout_branch_and_update_ui(&self, branch_to_checkout_to: String, is_remote: bool) {
        let selected_repository = self.imp().selected_repository.take();
        self.imp().selected_repository.replace(
            SelectedRepository::try_fetching_selected_repository(
                &selected_repository.user_repository,
            )
            .unwrap(),
        );

        let (result_sender, result_receiver) =
            MainContext::channel::<Result<(), String>>(Priority::default());

        self.toggle_git_action_button(false);

        thread::spawn(move || {
            let result_sender = result_sender.clone();

            match RepositoryUtils::checkout_branch(
                &selected_repository.git_repository.as_ref().unwrap(),
                &branch_to_checkout_to,
                is_remote,
            ) {
                Ok(_) => result_sender
                    .send(Ok(()))
                    .expect("Could not send result through channel"),
                Err(error) => result_sender
                    .send(Err(error.to_string()))
                    .expect("Could not send error through channel"),
            };
        });

        result_receiver.attach(
            None,
            clone!(
                @weak self as win => @default-return Continue(false),
                        move |result| {
                            win.toggle_git_action_button(true);

                            match result {
                                Ok(_) => win.update_pull_indication_box(0),
                                Err(error) => win.emit_by_name::<()>("error", &[&error.to_string()])
                            }
                            win.update_commits_sidebar();
                            Continue(true)
                        }
            ),
        );
    }

    /// Fetches the repository checked out branch, and update ui.
    fn fetch_repository_checked_out_branch_and_update_ui(
        &self,
        username: String,
        password: String,
        private_key_path: String,
        passphrase: String,
    ) {
        let selected_repository = self.imp().selected_repository.take();

        let selected_repository_path = selected_repository.user_repository.path.clone();

        self.imp().selected_repository.replace(selected_repository);

        let repository = Repository::open(selected_repository_path).unwrap();

        let (sender, receiver) =
            MainContext::channel::<Result<FetchResult, git2::Error>>(Priority::default());

        self.toggle_git_action_button(false);

        thread::spawn(move || {
            let sender = sender.clone();

            let fetch = fetch_checked_out_branch(
                &repository,
                username,
                password,
                private_key_path,
                passphrase,
            );

            sender.send(fetch).expect("Could not send through channel");
        });

        receiver.attach(
            None,
            clone!(@weak self as win => @default-return Continue(false),
                    move |fetch| {
                        match fetch {
                            Ok(fetch_result) => win.update_git_action_button(fetch_result),
                            Err(error) => {
                                win.try_to_find_correct_git_button_action();
                                win.toggle_git_action_button(true);
                                win.emit_by_name::<()>("error", &[&error.to_string()])
                            },
                        }
                        Continue(true)
                    }
            ),
        );
    }

    /// Used to discard a folder and update UI.
    pub fn discard_folder_and_update_ui(&self, folder_path: &str) {
        let selected_repository = self.imp().selected_repository.take();
        self.imp().selected_repository.replace(
            SelectedRepository::try_fetching_selected_repository(
                &selected_repository.user_repository,
            )
            .unwrap(),
        );

        let folder_files: Vec<ChangedFile>;
        {
            let file_tree = self.imp().sidebar.imp().changed_files.borrow();
            folder_files = file_tree.get_files_of_folder(folder_path);
        }

        match RepositoryUtils::discard_folder(
            &selected_repository.git_repository.unwrap(),
            &folder_files,
        ) {
            Ok(_) => {
                self.update_commits_sidebar();
            }
            Err(error) => self.emit_by_name::<()>("error", &[&error.to_string()]),
        };
    }

    /// Fetches the repository checked out branch, and update ui.
    fn try_fetch_without_auth_and_update_ui(&self) {
        let selected_repository = self.imp().selected_repository.take();

        let selected_repository_path = selected_repository.user_repository.path.clone();

        self.imp().selected_repository.replace(selected_repository);

        let repository = Repository::open(selected_repository_path).unwrap();

        let (sender, receiver) =
            MainContext::channel::<Result<FetchResult, git2::Error>>(Priority::default());

        self.toggle_git_action_button(false);

        thread::spawn(move || {
            let sender = sender.clone();

            let fetch = fetch_checked_out_branch(
                &repository,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            );

            sender.send(fetch).expect("Could not send through channel");
        });

        receiver.attach(
            None,
            clone!(@weak self as win => @default-return Continue(false),
                        move |fetch| {
                            match fetch {
                                Ok(fetch_result) => {
                                    win.update_git_action_button(fetch_result);
                                },
                                Err(error) => {
                                    // TODO: Manage errors.
                                    if (error.class() == git2::ErrorClass::Http)
                                        || (error.class() == git2::ErrorClass::Ssh)
                                    {
                                        win.do_git_action_with_auth_check(ActionType::FETCH);
                                    } else {
                                        win.emit_by_name::<()>("error", &[&error.to_string()]);
                                        win.update_commits_sidebar();
                                        win.try_to_find_correct_git_button_action();
                                        win.toggle_git_action_button(true);
                                    }
                                },
                            }
                            Continue(true)
                        }
            ),
        );
    }
}
