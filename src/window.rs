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

use std::{fs, path::PathBuf, thread};

use crate::{
    glib::clone,
    models::{bagit_git_profile::BagitGitProfile, bagit_repository::BagitRepository},
    utils::{
        self, action_type::ActionType, db::AppDatabase, profile_mode::ProfileMode,
        repository_utils::RepositoryUtils, selected_repository::SelectedRepository,
    },
    widgets::branches_dialog::BagitBranchesDialog,
    widgets::gpg_passphrase_dialog::BagitGpgPassphraseDialog,
    widgets::https_action_dialog::BagitHttpsActionDialog,
    widgets::{
        ssh_action_dialog::BagitSshActionDialog, ssh_passphrase_dialog::BagitSshPassphraseDialog,
    },
};
use adw::{
    subclass::prelude::*,
    traits::{ActionRowExt, MessageDialogExt, PreferencesRowExt},
};
use gettextrs::gettext;
use git2::Repository;
use gtk::{
    gio,
    glib::{self, MainContext, Priority},
    template_callbacks,
};
use gtk::{glib::closure_local, prelude::*};
use uuid::Uuid;

use crate::clone_repository_page::BagitCloneRepositoryPage;
use crate::repository_page::BagitRepositoryPage;
use crate::widgets::action_bar::BagitActionBar;
use crate::widgets::repositories::BagitRepositories;
use std::cell::RefCell;

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/window.ui")]
    pub struct BagitDesktopWindow {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub selection_button: TemplateChild<gtk::ToggleButton>,
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

        pub selected_repositories_ids_for_deletion: RefCell<Vec<Uuid>>,
    }

    #[template_callbacks]
    impl BagitDesktopWindow {
        #[template_callback]
        fn selection_button_toggled(&self, toggle_button: gtk::ToggleButton) {
            self.repositories_window
                .imp()
                .recent_repositories_revealer
                .set_reveal_child(!toggle_button.is_active());

            if toggle_button.is_active() {
                self.action_bar_content
                    .imp()
                    .action_stack
                    .set_visible_child_name("destructive action");
            } else {
                self.action_bar_content
                    .imp()
                    .action_stack
                    .set_visible_child_name("normal actions");
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BagitDesktopWindow {
        const NAME: &'static str = "BagitDesktopWindow";
        type Type = super::BagitDesktopWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
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
                        "repository page" => {
                            win.repository_page.update_commits_sidebar();
                            win.repository_page.imp().commit_view.update_git_profiles_list();
                            win.repository_page.update_branch_name();
                        },
                        "clone page" => win.clone_repository_page.update_git_profiles_list(
                            &win.app_database.get_all_git_profiles()
                        ),
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

        win.repositories_page_signals();
        win.repository_page_signals();
        win.action_bar_signals();
        win.clone_repository_page_signals();
        win.init_all_repositories();
        win.update_recent_repositories();
        win
    }

    /*
     * Signals send by the repositories page.
     */
    pub fn repositories_page_signals(&self) {
        self.imp().repositories_window.connect_closure(
            "open-repository",
            false,
            closure_local!(@watch self as win => move |
                _repositories: BagitRepositories,
                path: &str
                | {
                    // If we aren't in the selection mode, we go to the selected repository:
                    if !win.imp().selection_button.is_active() {
                        // We update the selected repository :
                        let found_repository = win.imp().app_database.get_repository_from_path(&path);
                        win.imp().repository_page.imp().sidebar.clear_changed_files_list();
                        win.imp().repository_page.imp().sidebar.imp().change_from_file.set(false);
                        win.imp().repository_page.imp().main_view_stack.set_visible_child_name("hello world");
                        win.imp().repository_page.imp().commit_view.update_commit_view(0);

                        if found_repository.is_some() {
                            let repo = found_repository.unwrap();

                            match SelectedRepository::try_fetching_selected_repository(&repo) {
                                Ok(selected_repository) => {
                                    let repo_id = selected_repository.user_repository.repository_id;

                                    thread::spawn(move ||{
                                            AppDatabase::init_database().update_last_opening_of_repository(repo_id);
                                        }
                                    );
                                    win.imp().repository_page.init_repository_page(selected_repository);
                                    win.imp().stack.set_visible_child_name("repository page");
                                }
                                Err(_) => {}
                            }
                        }
                    }
                }
            ),
        );
        self.imp().repositories_window.connect_closure(
            "search-event",
            false,
            closure_local!(@watch self as win => move |
                _repositories: BagitRepositories,
                search: &str
                | {
                    win.imp().repositories_window.clear_all_repositories_ui_list();
                    if search.is_empty() {
                        win.init_all_repositories();
                    } else {
                        win.find_repositories_with_search(search);
                    }
                }
            ),
        );
    }

    /// Used to build a new repository row.
    pub fn build_repository_row(&self, bagit_repository: &BagitRepository) -> adw::ActionRow {
        if !self.imp().repositories_window.is_visible() {
            self.imp().status_page.set_visible(false);
            self.imp().repositories_window.set_visible(true);
        }

        let full_path: String = format!("{}{}", "~", bagit_repository.path);

        let new_row: adw::ActionRow = adw::ActionRow::new();
        let row_image: gtk::Image = gtk::Image::builder().icon_name("go-next-symbolic").build();
        let check_button: gtk::CheckButton = gtk::CheckButton::new();
        check_button.add_css_class("selection-mode");

        let cloned_repo = bagit_repository.clone();

        check_button.connect_toggled(clone!(
            @weak self as win
            => move |button| {
                let mut selected_list = win.imp().selected_repositories_ids_for_deletion.take();
                if button.is_active() {
                    selected_list.push(cloned_repo.repository_id);
                } else {
                    let index = selected_list.iter().position(|&id| id == cloned_repo.repository_id).unwrap();
                    selected_list.remove(index);
                }
                win.imp().selected_repositories_ids_for_deletion.replace(selected_list);
            }
        ));

        let open_repo_revealer: gtk::Revealer = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::Crossfade)
            .child(&row_image)
            .reveal_child(true)
            .build();

        let delete_repo_button_revealer: gtk::Revealer = gtk::Revealer::builder()
            .transition_type(gtk::RevealerTransitionType::SlideLeft)
            .child(&check_button)
            .build();

        self.imp()
            .selection_button
            .bind_property("active", &open_repo_revealer, "reveal-child")
            .transform_to(|_, active: bool| Some(!active))
            .build();

        self.imp()
            .selection_button
            .bind_property("active", &delete_repo_button_revealer, "reveal-child")
            .build();
        new_row.set_title(&bagit_repository.name);
        new_row.set_subtitle(&full_path);
        new_row.set_height_request(64);
        new_row.add_prefix(&delete_repo_button_revealer);
        new_row.add_suffix(&open_repo_revealer);

        return new_row;
    }

    /// Add a new row to the list of all repositories.
    pub fn add_list_row_to_all_repositories(&self, bagit_repository: &BagitRepository) {
        let new_row = self.build_repository_row(bagit_repository);

        self.imp()
            .repositories_window
            .imp()
            .all_repositories
            .append(&new_row);
    }

    /// Add a new row to the list of recent repositories.
    pub fn add_list_row_to_recent_repositories(&self, bagit_repository: &BagitRepository) {
        let new_row = self.build_repository_row(bagit_repository);

        self.imp()
            .repositories_window
            .imp()
            .recent_repositories
            .append(&new_row);
    }

    /// Used to initialize the repositories.
    fn init_all_repositories(&self) {
        let all_repositories: Vec<BagitRepository> = self.imp().app_database.get_all_repositories();

        for repository in all_repositories {
            self.add_list_row_to_all_repositories(&repository);
        }
    }

    /// Used to initialize the repositories.
    fn update_recent_repositories(&self) {
        let recent_repositories: Vec<BagitRepository> =
            self.imp().app_database.get_recent_repositories();

        self.imp()
            .repositories_window
            .clear_recent_repositories_ui_list();

        for repository in recent_repositories {
            self.add_list_row_to_recent_repositories(&repository);
        }
    }

    /// Used to initialize the repositories.
    fn find_repositories_with_search(&self, search: &str) {
        let found_repositories: Vec<BagitRepository> = self
            .imp()
            .app_database
            .get_all_repositories_with_search(search);

        for repository in found_repositories {
            self.add_list_row_to_all_repositories(&repository);
        }
    }

    /**
     * Used for listenning to the clone button of the BagitActionBar widget.
     */
    fn action_bar_signals(&self) {
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

                        // We must check if the selected repository isn't already in the application:
                        match win2.imp().app_database.get_repository_from_path(&folder_path_str) {
                            Some(_) => {
                                let toast = adw::Toast::new(&gettext("_Repo already present"));
                                win2.imp().toast_overlay.add_toast(toast);
                                return;
                            },
                            None => {},
                        };

                        let repository = Repository::open(&folder_path);

                        // We make sure the selected folder is a valid repository.
                        match repository {
                            Ok(_) => {
                                let folder_name = RepositoryUtils::get_folder_name_from_os(folder_path_str);

                                let new_bagit_repository = BagitRepository::new(Uuid::new_v4(), folder_name, folder_path_str.to_string(), None);

                                win2.add_list_row_to_all_repositories(
                                    &new_bagit_repository
                                );
                                win2.imp().app_database.add_repository(
                                    &new_bagit_repository
                                );
                                win2.update_recent_repositories();
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
        self.imp().action_bar_content.connect_closure(
            "clone-repository",
            false,
            closure_local!(@watch self as win => move |_action_bar_content: BagitActionBar| {
                // We must make sure entry fields are blank when going to the clone page :
                win.imp().clone_repository_page.clear_page();
                // We update the list of git profiles in the page :
                let git_profiles: Vec<BagitGitProfile> = win.imp().app_database.get_all_git_profiles();
                for profile in git_profiles {
                    win.imp().clone_repository_page.add_git_profile_row(&profile);
                }
                win.imp().stack.set_visible_child_name("clone page");
            }),
        );
        self.imp().action_bar_content.connect_closure(
            "delete-selected-repositories",
            false,
            closure_local!(@watch self as win => move |_action_bar_content: BagitActionBar| {
                let selected_repositories = win.imp().selected_repositories_ids_for_deletion.take();
                let total_deleted = selected_repositories.len();

                for repository_id in selected_repositories {
                    win.imp().app_database.delete_repository(&repository_id.to_string());
                }

                let toast_text = if total_deleted == 0 {
                    gettext("_No deleted repositories")
                } else if total_deleted == 1 {
                    format!("{} {}", total_deleted, gettext("_Deleted repository"))
                } else {
                    format!("{} {}", total_deleted, gettext("_Deleted repositories"))
                };

                win.imp().repositories_window.clear_all_repositories_ui_list();
                win.init_all_repositories();
                win.update_recent_repositories();
                win.imp().selection_button.set_active(false);

                let toast = adw::Toast::new(&toast_text);
                win.imp().toast_overlay.add_toast(toast);
                return;
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
                        .title(gettext("_Select private key path"))
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
            "clone-repository",
            false,
            closure_local!(@watch self as win => move |
                clone_repository_page: BagitCloneRepositoryPage,
                url: &str,
                location: &str
                | {
                    let (error_sender, error_receiver) = MainContext::channel::<String>(Priority::default());
                    let (result_sender, result_receiver) = MainContext::channel::<(String, String)>(Priority::default());

                    let url_copy = url.to_owned();
                    let location_copy = location.to_owned();

                    let borrowed_profile_mode = clone_repository_page.imp().profile_mode.take();
                    clone_repository_page.imp().profile_mode.replace(
                        borrowed_profile_mode.clone()
                    );

                    let selected_profile: Option<BagitGitProfile> = match borrowed_profile_mode {
                        ProfileMode::SelectedProfile(profile) => Some(profile),
                        _ => None
                    };
                    let (
                        username,
                        password,
                        private_key_path,
                        passphrase,
                    ) =  match &selected_profile {
                        None => (
                            "".to_string(),
                            "".to_string(),
                            "".to_string(),
                            win.imp().clone_repository_page.imp().passphrase.text().to_string(),
                        ),
                        Some(profile) => (
                            profile.username.to_string(),
                            profile.password.to_string(),
                            profile.private_key_path.to_string(),
                            clone_repository_page.imp().passphrase.text().to_string(),
                        ),
                    };
                    thread::spawn(move || {
                        let error_sender = error_sender.clone();
                        let result_sender = result_sender.clone();

                        let callback = RepositoryUtils::find_correct_callback(
                            url_copy.clone(),
                            username,
                            password,
                            passphrase,
                            private_key_path
                        );

                        let new_path = RepositoryUtils::create_new_folder_path(&url_copy, &location_copy);

                        let new_folder = fs::create_dir(&new_path);
                        match new_folder {
                            Ok(_) => {
                                match RepositoryUtils::clone_repository(&url_copy, &new_path, callback) {
                                    Ok(repository) => {
                                        match selected_profile {
                                            Some(profile) => {
                                                // Once the repository is cloned, we update it's config file:
                                                match RepositoryUtils::override_git_config(&repository, &profile) {
                                                    Ok(_) => result_sender.send(
                                                        (
                                                            RepositoryUtils::get_folder_name_from_os(&url_copy).to_string(),
                                                            new_path
                                                        )
                                                    ).expect("Could not send result through channel"),
                                                    Err(error) => error_sender.send(error.to_string()).expect("Could not send error through channel")
                                                };
                                            },
                                            None => result_sender.send(
                                                (
                                                    RepositoryUtils::get_folder_name_from_os(&url_copy).to_string(),
                                                    new_path
                                                )
                                            ).expect("Could not send result through channel")
                                        }
                                    }
                                    Err(e) => {
                                        // We must make sure to delete the created folder !
                                        let removed_directory = fs::remove_dir_all(&new_path);
                                        match removed_directory {
                                            Ok(_) => error_sender.send(e.to_string()).expect("Could not send error through channel"),
                                            Err(error) => error_sender.send(error.to_string()).expect("Could not send error through channel")
                                        }
                                    }
                                }
                            },
                            Err(e) => error_sender.send(e.to_string()).expect("Could not send error through channel")
                        }
                    });

                    error_receiver.attach(
                        None,
                        clone!(@weak win as win2 => @default-return Continue(false),
                                    move |error| {
                                        win2.imp().clone_repository_page.to_main_page();
                                        win2.show_error_dialog(&error);
                                        Continue(true)
                                    }
                        ),
                    );

                    result_receiver.attach(
                        None,
                        clone!(
                            @weak win as win2 => @default-return Continue(false),
                                    move |elements| {
                                        let mut new_repository = BagitRepository::new(Uuid::new_v4(), elements.0, elements.1, None);

                                        win2.save_clone_repository(&mut new_repository);
                                        win2.imp().clone_repository_page.to_main_page();
                                        Continue(true)
                                    }
                        ),
                    );
                }
            ),
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
                private_key_path: &str,
                signing_key: &str
                | {
                    let (error_sender, error_receiver) = MainContext::channel::<String>(Priority::default());
                    let (result_sender, result_receiver) = MainContext::channel::<(String, String)>(Priority::default());

                    let url_copy = url.to_owned();
                    let location_copy = location.to_owned();

                    let passphrase = win.imp().clone_repository_page.imp().passphrase.text().to_string();
                    let username_copy = username.to_owned();
                    let password_copy = password.to_owned();
                    let private_key_path_copy = private_key_path.to_owned();

                    let profile_id = Uuid::new_v4();

                    // We make sure that the profile name is unique :
                    let same_profile_name_number = win.imp().app_database.get_number_of_git_profiles_with_name(
                        &profile_name,
                        &profile_id.to_string()
                    );
                    let final_profil_name : String =  if same_profile_name_number != 0 {
                        let new_name = format!("{} ({})", profile_name, same_profile_name_number);
                        new_name
                    } else {
                        profile_name.to_string()
                    };


                    let new_profile = BagitGitProfile::new(
                        profile_id,
                        final_profil_name,
                        email.to_string(),
                        username.to_string(),
                        password.to_string(),
                        private_key_path.to_string(),
                        signing_key.to_string()
                    );
                    win.imp().app_database.add_git_profile(&new_profile);

                    thread::spawn(move || {
                        let error_sender = error_sender.clone();
                        let result_sender = result_sender.clone();

                        let callback = RepositoryUtils::find_correct_callback(
                            url_copy.clone(),
                            username_copy,
                            password_copy,
                            passphrase,
                            private_key_path_copy
                        );
                        let new_path = RepositoryUtils::create_new_folder_path(&url_copy, &location_copy);

                        let new_folder = fs::create_dir(&new_path);
                        match new_folder {
                            Ok(_) => {
                                match RepositoryUtils::clone_repository(&url_copy, &new_path, callback) {
                                    Ok(repository) => {
                                        // Once the repository is cloned, we update it's config file:
                                        match RepositoryUtils::override_git_config(&repository, &new_profile) {
                                            Ok(_) => result_sender.send(
                                                (
                                                    RepositoryUtils::get_folder_name_from_os(&url_copy).to_string(),
                                                    new_path
                                                )
                                            ).expect("Could not send result through channel"),
                                            Err(error) => error_sender.send(error.to_string()).expect("Could not send error through channel")
                                        };
                                    },
                                    Err(e) => {
                                        // We must make sure to delete the created folder !
                                        let removed_directory = fs::remove_dir_all(&new_path);
                                        match removed_directory {
                                            Ok(_) => error_sender.send(e.to_string()).expect("Could not send error through channel"),
                                            Err(error) => error_sender.send(error.to_string()).expect("Could not send error through channel")
                                        }
                                    }
                                }
                            },
                            Err(e) => {
                                error_sender.send(e.to_string()).expect("Could not send error through channel")
                            }
                        }
                    });

                    error_receiver.attach(
                        None,
                        clone!(@weak win as win2 => @default-return Continue(false),
                                    move |error| {
                                        win2.imp().clone_repository_page.to_main_page();
                                        win2.show_error_dialog(&error);
                                        Continue(true)
                                    }
                        ),
                    );

                    result_receiver.attach(
                        None,
                        clone!(
                            @weak win as win2 => @default-return Continue(false),
                                    move |elements| {
                                        let mut new_repository = BagitRepository::new(Uuid::new_v4(), elements.0, elements.1, None);

                                        win2.save_clone_repository(&mut new_repository);
                                        win2.imp().clone_repository_page.to_main_page();
                                        Continue(true)
                                    }
                        ),
                    );
                }
            ),
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
                win.update_recent_repositories();
            }),
        );
        self.imp().repository_page.connect_closure(
            "error",
            false,
            closure_local!(@watch self as win => move |
                _repository_page: BagitRepositoryPage,
                error_message: &str
                | {
                win.show_error_dialog(error_message);
            }),
        );
        self.imp().repository_page.connect_closure(
            "select-branch",
            false,
            closure_local!(@watch self as win => move |
                _repository_page: BagitRepositoryPage,
                repo_path: &str,
                | {
                    let ctx: MainContext = glib::MainContext::default();
                    let repo_path_clone = String::from(repo_path);
                    ctx.spawn_local(clone!(@weak win as win2 => async move {
                        let branch_dialog = BagitBranchesDialog::new(repo_path_clone.to_string());
                        branch_dialog.set_transient_for(Some(&win2));
                        branch_dialog.set_modal(true);
                        branch_dialog.present();
                        branch_dialog.fetch_branches();

                        branch_dialog.connect_closure("select-branch", false, closure_local!(
                            @watch win2 as win3 =>
                            move |
                            branch_dialog: BagitBranchesDialog,
                            branch_name: &str,
                            is_remote: bool
                            | {
                                branch_dialog.close();
                                win3.show_checkout_dialog(branch_name.to_string(), is_remote);
                            }
                        ));
                    }
                ));
            }),
        );
        self.imp().repository_page.connect_closure(
            "commit-files-with-signing-key",
            false,
            closure_local!(@watch self as win => move |
                repository_page: BagitRepositoryPage,
                author: &str,
                author_email: &str,
                message: &str,
                signing_key: &str,
                description: &str,
                need_to_save_profile: bool
                | {
                    let ctx: MainContext = glib::MainContext::default();
                    let cloned_signing_key = String::from(signing_key);
                    let cloned_message = String::from(message);
                    let cloned_author = String::from(author);
                    let cloned_author_email = String::from(author_email);
                    let cloned_description = String::from(description);
                    let cloned_need_to_save_profile = need_to_save_profile.clone();
                    ctx.spawn_local(clone!(@weak win as win2 => async move {
                        let passphrase_dialog: BagitGpgPassphraseDialog = BagitGpgPassphraseDialog::new(&cloned_signing_key);
                        passphrase_dialog.set_transient_for(Some(&win2));
                        passphrase_dialog.set_modal(true);
                        passphrase_dialog.present();

                        passphrase_dialog.connect_closure("fetch-passphrase", false, closure_local!(
                            move |
                            passphrase_dialog: BagitGpgPassphraseDialog,
                            passphrase: &str
                            | {
                                passphrase_dialog.close();
                                repository_page.commit_files_and_update_ui(
                                    &cloned_author,
                                    &cloned_author_email,
                                    &cloned_message,
                                    &cloned_signing_key,
                                    &passphrase,
                                    &cloned_description,
                                    cloned_need_to_save_profile,
                                );
                            }
                        ));
                    }
                ));
            }),
        );
        self.imp().repository_page.connect_closure(
            "missing-ssh-information",
            false,
            closure_local!(@watch self as win => move |
                repository_page: BagitRepositoryPage,
                username: &str,
                private_key_path: &str,
                action_type: ActionType
                | {
                    let ctx: MainContext = glib::MainContext::default();
                    let cloned_username = String::from(username);
                    let cloned_private_key_path= String::from(private_key_path);
                    ctx.spawn_local(clone!(@weak win as win2 => async move {
                        let dialog: BagitSshActionDialog = BagitSshActionDialog::new(&cloned_username, &cloned_private_key_path);
                        dialog.set_transient_for(Some(&win2));
                        dialog.set_modal(true);
                        dialog.present();

                        dialog.connect_closure("select-location", false, closure_local!(
                            @watch win2 as win3 => move |
                            ssh_dialog: BagitSshActionDialog,
                            | {
                                let ctx: MainContext = glib::MainContext::default();
                                ctx.spawn_local(clone!(@weak win3 as win4 => async move {
                                    let dialog = gtk::FileDialog::builder()
                                        .accept_label(gettext("_Add"))
                                        .modal(true)
                                        .title(gettext("_Select private key path"))
                                        .build();

                                    if let Ok(res) = dialog.open_future(Some(&win4)).await {
                                        ssh_dialog.imp().private_key_path.set_text(
                                            res.path().unwrap_or(PathBuf::new()).to_str().unwrap()
                                        );
                                    }
                                }));
                            }
                        ));

                        dialog.connect_closure("push-with-ssh-informations", false, closure_local!(
                            @watch repository_page =>
                            move |
                            dialog: BagitSshActionDialog,
                            username: &str,
                            private_key_path: &str,
                            passphrase: &str,
                            | {
                                dialog.close();
                                repository_page.do_git_action_with_information(
                                    String::from(username),
                                    String::new(),
                                    String::from(private_key_path),
                                    String::from(passphrase),
                                    action_type
                                );
                            }
                        ));
                    }
                ));
            }),
        );
        self.imp().repository_page.connect_closure(
            "ssh-passphrase-dialog",
            false,
            closure_local!(@watch self as win => move |
                repository_page: BagitRepositoryPage,
                username: &str,
                private_key_path: &str,
                action_type: ActionType
                | {
                    let ctx: MainContext = glib::MainContext::default();

                    let cloned_username = String::from(username);
                    let cloned_private_key_path= String::from(private_key_path);

                    ctx.spawn_local(clone!(@weak win as win2, => async move {
                        let dialog: BagitSshPassphraseDialog = BagitSshPassphraseDialog::new(cloned_username, cloned_private_key_path);
                        dialog.set_transient_for(Some(&win2));
                        dialog.set_modal(true);
                        dialog.present();

                        dialog.connect_closure("push-with-passphrase", false, closure_local!(
                            @watch repository_page =>
                            move |
                            dialog: BagitSshPassphraseDialog,
                            username: &str,
                            private_key_path: &str,
                            passphrase: &str,
                            | {
                                dialog.close();
                                repository_page.do_git_action_with_information(
                                    String::from(username),
                                    String::new(),
                                    String::from(private_key_path),
                                    String::from(passphrase),
                                    action_type
                                );
                            }
                        ));
                    }
                ));
            }),
        );
        self.imp().repository_page.connect_closure(
            "missing-https-information",
            false,
            closure_local!(@watch self as win => move |
                repository_page: BagitRepositoryPage,
                username: &str,
                password: &str,
                action_type: ActionType
                | {
                    let ctx: MainContext = glib::MainContext::default();
                    let cloned_username = String::from(username);
                    let cloned_password = String::from(password);
                    ctx.spawn_local(clone!(@weak win as win2 => async move {
                        let dialog: BagitHttpsActionDialog = BagitHttpsActionDialog::new(&cloned_username, &cloned_password);
                        dialog.set_transient_for(Some(&win2));
                        dialog.set_modal(true);
                        dialog.present();

                        dialog.connect_closure("push-with-https-informations", false, closure_local!(
                            @watch repository_page =>
                            move |
                            dialog: BagitHttpsActionDialog,
                            username: &str,
                            password: &str
                            | {
                                dialog.close();
                                repository_page.do_git_action_with_information(
                                    String::from(username),
                                    String::from(password),
                                    String::new(),
                                    String::new(),
                                    action_type
                                );
                            }
                        ));
                    }
                ));
            }),
        );
    }

    /// Used to show validate dialog for switching branch.
    pub fn show_checkout_dialog(&self, branch_name: String, is_remote: bool) {
        let dialog_body = format!("{} {}", gettext("_Changes will be brought to"), branch_name);
        let ctx: MainContext = glib::MainContext::default();
        ctx.spawn_local(clone!(@weak self as win => async move {
            let branch_dialog = adw::MessageDialog::builder()
            .modal(true)
            .transient_for(&win)
            .heading(&gettext("_Change branch dialog title"))
            .body(&dialog_body)
            .build();

            branch_dialog.add_response("cancel", &gettext("_Cancel"));
            branch_dialog.add_response("validate", &gettext("_Validate"));


            branch_dialog.connect_response(None,clone!(
                @weak win as win2,
                => move |_, response| {
                    match response {
                        "validate" => {
                            win2.imp().repository_page.checkout_branch_and_update_ui(branch_name.to_owned(), is_remote);
                        },
                        _ => {}
                    }
                }
            ));

            branch_dialog.present();
        }));
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

    /// Used to save a new cloned repository
    pub fn save_clone_repository(&self, new_repository: &mut BagitRepository) {
        self.add_list_row_to_all_repositories(&new_repository);

        let profile_id: Option<Uuid> = match self
            .imp()
            .clone_repository_page
            .imp()
            .profile_mode
            .borrow()
            .get_profile_mode()
        {
            ProfileMode::SelectedProfile(profile) => Some(profile.profile_id),
            _ => None,
        };

        new_repository.git_profile_id = profile_id;

        self.imp().app_database.add_repository(&new_repository);
        self.update_recent_repositories();
        self.imp().stack.set_visible_child_name("main page");
    }
}
