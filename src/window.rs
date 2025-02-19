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
    create_repository_page::BagitCreateRepositoryPage,
    glib::clone,
    models::{bagit_git_profile::BagitGitProfile, bagit_repository::BagitRepository},
    utils::{
        action_type::ActionType, db::AppDatabase, profile_mode::ProfileMode,
        repository_utils::RepositoryUtils, selected_repository::SelectedRepository,
    },
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
    use std::collections::HashMap;

    use crate::create_repository_page::BagitCreateRepositoryPage;

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
        pub create_repository_page: TemplateChild<BagitCreateRepositoryPage>,
        #[template_child]
        pub clone_repository_page: TemplateChild<BagitCloneRepositoryPage>,
        #[template_child]
        pub repository_page: TemplateChild<BagitRepositoryPage>,

        pub app_database: RefCell<AppDatabase>,

        pub selected_repositories_ids_for_deletion: RefCell<Vec<Uuid>>,

        pub ssh_passphrases: RefCell<HashMap<String, String>>,
        pub gpg_passphrases: RefCell<HashMap<String, String>>,
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
                    let app_database = win.app_database.take();

                    match win.stack.visible_child_name().unwrap().as_str() {
                        "repository page" => {
                            let ssh_passphrases = win.ssh_passphrases.take();
                            win.repository_page.imp().ssh_passphrases.replace(ssh_passphrases.clone()); //TODO: MARKER
                            win.ssh_passphrases.replace(ssh_passphrases);
                            win.repository_page.update_repository_page();
                        },
                        "create repository page" => {
                            let git_profiles;

                            match app_database.get_all_git_profiles() {
                                Ok(profiles) => git_profiles = profiles,
                                Err(error) => {
                                    // TODO: Show error (maybe with a toast).

                                    tracing::warn!("Could not get all Git profiles: {}", error);

                                    return;
                                },
                            }

                            win.create_repository_page.update_git_profiles_list(
                                &git_profiles
                            )
                        },
                        "clone page" => {
                            let git_profiles;

                            match app_database.get_all_git_profiles() {
                                Ok(profiles) => git_profiles = profiles,
                                Err(error) => {
                                    // TODO: Show error (maybe with a toast).

                                    tracing::warn!("Could not get all Git profiles: {}", error);

                                    return;
                                },
                            }

                            win.clone_repository_page.update_git_profiles_list(
                                &git_profiles
                            )
                        },
                        _ => {}
                    }

                    win.app_database.replace(app_database);
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

        let mut app_database = win.imp().app_database.take();

        app_database.init_database();

        win.imp().app_database.replace(app_database);

        win.repositories_page_signals();
        win.repository_page_signals();
        win.action_bar_signals();
        win.create_repository_button_signal();
        win.clone_button_signal();
        win.create_repository_page_signals();
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
                        // We update the selected repository:
                        let found_repository;

                        let app_database = win.imp().app_database.take();

                        match app_database.get_repository_from_path(&path) {
                            Ok(repo) => found_repository = repo,
                            Err(error) => {
                                // TODO: Show error (maybe with a toast).
                                tracing::warn!("Could not get repository from path: {}", error);

                                return;
                            }
                        }

                        win.imp().app_database.replace(app_database);

                        if found_repository.is_some() {
                            let repo = found_repository.unwrap();

                            match SelectedRepository::try_fetching_selected_repository(&repo) {
                                Ok(selected_repository) => {
                                    win.imp().repository_page.init_repository_page(selected_repository);
                                    win.imp().stack.set_visible_child_name("repository page");
                                    return;
                                }
                                Err(_) => {
                                    let toast = adw::Toast::new(&gettext("_Repository doesn't exist anymore"));
                                    win.imp().toast_overlay.add_toast(toast);
                                }
                            }
                        }
                        win.imp().repositories_window.imp().recent_repositories.unselect_all();
                        win.imp().repositories_window.imp().all_repositories.unselect_all();
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

    pub fn save_ssh_passphrase(&self, key_path: String, passphrase: String) {
        let mut ssh_passphrases = self.imp().ssh_passphrases.take();

        ssh_passphrases.insert(key_path, passphrase);

        self.imp().ssh_passphrases.replace(ssh_passphrases);
    }

    pub fn save_gpg_passphrase(&self, key: String, passphrase: String) {
        let mut gpg_passphrases = self.imp().gpg_passphrases.take();

        gpg_passphrases.insert(key, passphrase);

        self.imp().gpg_passphrases.replace(gpg_passphrases);
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
        let all_repositories;

        let app_database = self.imp().app_database.take();

        match app_database.get_all_repositories() {
            Ok(repositories) => all_repositories = repositories,
            Err(error) => {
                // TODO: Show error (maybe with a toast).

                tracing::warn!("Could not get all repositories: {}", error);

                return;
            }
        }

        self.imp().app_database.replace(app_database);

        if all_repositories.is_empty() {
            self.imp().repositories_window.set_visible(false);
            self.imp().status_page.set_visible(true);
        }

        for repository in all_repositories {
            self.add_list_row_to_all_repositories(&repository);
        }
    }

    /// Used to initialize the repositories.
    fn update_recent_repositories(&self) {
        let recent_repositories;

        let app_database = self.imp().app_database.take();

        match app_database.get_recent_repositories() {
            Ok(repositories) => recent_repositories = repositories,
            Err(error) => {
                // TODO: Show error (maybe with a toast).

                tracing::warn!("Could not get recent repositories: {}", error);

                return;
            }
        }

        self.imp().app_database.replace(app_database);

        self.imp()
            .repositories_window
            .clear_recent_repositories_ui_list();

        for repository in recent_repositories {
            self.add_list_row_to_recent_repositories(&repository);
        }
    }

    /// Used to initialize the repositories.
    fn find_repositories_with_search(&self, search: &str) {
        let found_repositories;

        let app_database = self.imp().app_database.take();

        match app_database.get_all_repositories_with_search(search) {
            Ok(repositories) => found_repositories = repositories,
            Err(error) => {
                // TODO: Show error (maybe with a toast).

                tracing::warn!("Could not get repositories with search: {}", error);

                return;
            }
        }

        self.imp().app_database.replace(app_database);

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

                        let repository;

                        let app_database = win2.imp().app_database.take();

                        match app_database.get_repository_from_path(&folder_path_str) {
                            Ok(repo) => repository = repo,
                            Err(error) => {
                                // TODO: Show error (maybe with a toast).
                                tracing::warn!("Could not get repository from path: {}", error);

                                return;
                            }
                        }

                        win2.imp().app_database.replace(app_database);

                        // We must check if the selected repository isn't already in the application:
                        match repository {
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

                                let app_database = win2.imp().app_database.take();

                                if let Err(error) = app_database.add_repository(
                                    &new_bagit_repository
                                ) {
                                    tracing::warn!("Could not add repository: {}", error);

                                    let toast = adw::Toast::new(&gettext("_Could not add repository"));
                                    win2.imp().toast_overlay.add_toast(toast);
                                }

                                win2.imp().app_database.replace(app_database);

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
    }

    /// Listener for the "create a repository" button of the BagitActionBar widget.
    fn create_repository_button_signal(&self) {
        self.imp().action_bar_content.connect_closure(
            "go-to-create-repository-page",
            false,
            closure_local!(@watch self as win => move |_action_bar_content: BagitActionBar| {
                win.imp().create_repository_page.clear_page();

                let git_profiles;

                let app_database = win.imp().app_database.take();

                match app_database.get_all_git_profiles() {
                    Ok(profiles) => git_profiles = profiles,
                    Err(error) => {
                        // TODO: Show error (maybe with a toast).

                        tracing::warn!("Could not get all Git profiles: {}", error);

                        return;
                    },
                }

                win.imp().app_database.replace(app_database);

                for profile in git_profiles {
                    win.imp().create_repository_page.add_git_profile_row(&profile);
                }

                win.imp().stack.set_visible_child_name("create repository page");
            }),
        );
    }

    fn create_repository_page_signals(&self) {
        self.imp().create_repository_page.connect_closure(
            "go-back", 
            false,
            closure_local!(@watch self as win => move |_create_repository_page: BagitCreateRepositoryPage| {
                win.imp().stack.set_visible_child_name("main page");
            })
        );

        self.imp().create_repository_page.connect_closure(
            "select-location", 
            false,
            closure_local!(@watch self as win => move |_create_repository_page: BagitCreateRepositoryPage| {
                let ctx: MainContext = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak win as win2 => async move {
                    let dialog = gtk::FileDialog::builder()
                        .accept_label(gettext("_Add"))
                        .modal(true)
                        .title(gettext("_Select location"))
                        .build();

                    if let Ok(res) = dialog.select_folder_future(Some(&win2)).await {
                        win2.imp().create_repository_page.imp().location_row.set_text(
                            res.path().unwrap_or(PathBuf::new()).to_str().unwrap()
                        );
                    }
                }));
            })
        );

        self.imp().create_repository_page.connect_closure(
            "select-private-key", 
            false,
            closure_local!(@watch self as win => move |_create_repository_page: BagitCreateRepositoryPage| {
                let ctx: MainContext = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak win as win2 => async move {
                    let dialog = gtk::FileDialog::builder()
                        .accept_label(gettext("_Add"))
                        .modal(true)
                        .title(gettext("_Select Private key path"))
                        .build();

                    if let Ok(res) = dialog.open_future(Some(&win2)).await {
                        win2.imp().create_repository_page.imp().private_key_path.set_text(
                            res.path().unwrap_or(PathBuf::new()).to_str().unwrap()
                        );
                    }
                }));
            })
        );

        self.imp().create_repository_page.connect_closure(
            "create-repository",
            false,
            closure_local!(@watch self as win => move |
                create_repository_page: BagitCreateRepositoryPage,
                repository_name: &str,
                location: &str
                | {
                    let repository_path = format!("{}/{}", location, repository_name);

                    let repository;

                    match fs::create_dir(&repository_path) {
                        Ok(_) => {
                            match Repository::init(&repository_path) {
                                Ok(repo) => repository = repo,
                                Err(error) => {
                                    win.show_error_dialog(&error.to_string());

                                    tracing::warn!("Could not create new repository: {}", error);

                                    return;
                                },
                            };
                        },
                        Err(error) => {
                            win.show_error_dialog(&error.to_string());

                            tracing::warn!("Could not create new directory in order to create new repository: {}", error);

                            return;
                        },
                    }

                    let borrowed_profile_mode = create_repository_page.imp().profile_mode.take();

                    let selected_profile: Option<BagitGitProfile> = match borrowed_profile_mode {
                        ProfileMode::SelectedProfile(profile) => Some(profile),
                        _ => None
                    };

                    match selected_profile {
                        Some(profile) => {
                            // Once the repository is created, we update its config file:
                            match RepositoryUtils::override_git_config(&repository, &profile) {
                                Ok(_) => tracing::debug!("Override of git config successful."),
                                Err(error) =>  tracing::warn!("Could not override git config: {}", error)
                            };
                        },
                        None => {},
                    }

                    let mut new_repository = BagitRepository::new(Uuid::new_v4(), repository_name.to_string(), repository_path, None);

                    let profile_mode =  create_repository_page.imp().profile_mode.take();

                    create_repository_page.imp().profile_mode.replace(profile_mode.clone());

                    win.save_repository(&mut new_repository, profile_mode);

                    win.imp().stack.set_visible_child_name("main page");
                }
            ),
        );

        self.imp().create_repository_page.connect_closure(
            "create-repository-and-add-profile",
            false,
            closure_local!(@watch self as win => move |
                create_repository_page: BagitCreateRepositoryPage,
                repository_name: &str,
                location: &str,
                profile_name: &str,
                email: &str,
                username: &str,
                password: &str,
                private_key_path: &str,
                signing_key: &str
                | {
                    let profile_id = Uuid::new_v4();

                    // We make sure that the profile name is unique:
                    let same_profile_name_number;

                    let app_database = win.imp().app_database.take();

                    match app_database.get_number_of_git_profiles_with_name(&profile_name, &profile_id.to_string()) {
                        Ok(number) => same_profile_name_number = number,
                        Err(error) => {
                            // TODO: Show error (maybe with a toast).

                            tracing::warn!("Could not get number of git profiles with name: {}", error);

                            return;
                        },
                    }

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

                    if let Err(error) = app_database.add_git_profile(&new_profile) {
                        tracing::warn!("Could not add Git profile: {}", error);

                        let toast = adw::Toast::new(&gettext("_Could not add Git profile"));
                        win.imp().toast_overlay.add_toast(toast);
                    }

                    win.imp().app_database.replace(app_database);

                    create_repository_page.emit_by_name::<()>("create-repository", &[&repository_name, &location]);
                }
            ),
        );
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
                let git_profiles;

                let app_database = win.imp().app_database.take();

                match app_database.get_all_git_profiles() {
                    Ok(profiles) => git_profiles = profiles,
                    Err(error) => {
                        // TODO: Show error (maybe with a toast).

                        tracing::warn!("Could not get all Git profiles: {}", error);

                        return;
                    },
                }

                win.imp().app_database.replace(app_database);

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

                let app_database = win.imp().app_database.take();

                for repository_id in selected_repositories {
                    if let Err(error) = app_database.delete_repository(&repository_id.to_string()) {
                        tracing::warn!("Could not delete repository: {}", error);

                        let toast = adw::Toast::new(&gettext("_Could not delete repository"));
                        win.imp().toast_overlay.add_toast(toast);
                    }
                }

                win.imp().app_database.replace(app_database);

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
                    let (sender, receiver) = MainContext::channel::<Result<(String, String), String>>(Priority::default());

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

                    let ssh_key_path;
                    let ssh_passphrase;

                    let (
                        username,
                        password,
                        private_key_path,
                        passphrase,
                    ) =  match &selected_profile {
                        None => {
                            ssh_key_path = String::from("");
                            ssh_passphrase = win.imp().clone_repository_page.imp().passphrase.text().to_string();

                            (
                                "".to_string(),
                                "".to_string(),
                                "".to_string(),
                                win.imp().clone_repository_page.imp().passphrase.text().to_string(),
                            )
                        },
                        Some(profile) => {
                            ssh_key_path = profile.private_key_path.to_string();
                            ssh_passphrase = clone_repository_page.imp().passphrase.text().to_string();

                            (
                                profile.username.to_string(),
                                profile.password.to_string(),
                                profile.private_key_path.to_string(),
                                clone_repository_page.imp().passphrase.text().to_string(),
                            )
                        }
                    };

                    if !ssh_key_path.is_empty() {
                        win.save_ssh_passphrase(ssh_key_path, ssh_passphrase);
                    }

                    thread::spawn(move || {
                        let sender = sender.clone();

                        let callback = RepositoryUtils::find_correct_callback(
                            url_copy.clone(),
                            username,
                            password,
                            passphrase,
                            private_key_path
                        );

                        let new_path = RepositoryUtils::create_new_folder_path(&url_copy, &location_copy);

                        let new_folder = fs::create_dir(&new_path);

                        if let Err(error) = new_folder {
                            sender.send(Err(error.to_string())).expect("Could not send error through channel");

                            return;
                        }

                        let repository: Repository;

                        match RepositoryUtils::clone_repository(&url_copy, &new_path, callback) {
                            Ok(repo) => {repository = repo}
                            Err(e) => {
                                // We must make sure to delete the created folder !
                                let removed_directory = fs::remove_dir_all(&new_path);
                                match removed_directory {
                                    Ok(_) => sender.send(Err(e.to_string())).expect("Could not send error through channel"),
                                    Err(error) => sender.send(Err(error.to_string())).expect("Could not send error through channel")
                                }

                                return;
                            }
                        }

                        let profile: BagitGitProfile;

                        match selected_profile {
                            Some(prof) => profile = prof,
                            None => {
                                sender.send(Ok(
                                    (
                                        RepositoryUtils::get_folder_name_from_os(&url_copy).to_string(),
                                        new_path
                                    )
                                )).expect("Could not send result through channel");

                                return;
                            },
                        }

                        // Once the repository is cloned, we update it's config file:
                        if let Err(error) = RepositoryUtils::override_git_config(&repository, &profile) {
                            sender.send(Err(error.to_string())).expect("Could not send error through channel");

                            return;
                        }

                        sender.send(
                            Ok((
                                RepositoryUtils::get_folder_name_from_os(&url_copy).to_string(),
                                new_path
                            ))
                        ).expect("Could not send result through channel")
                    });

                    receiver.attach(
                        None,
                        clone!(
                            @weak win as win2 => @default-return Continue(false),
                                    move |result| {
                                        match result {
                                            Ok(elements) => {
                                                let mut new_repository = BagitRepository::new(Uuid::new_v4(), elements.0, elements.1, None);

                                                let profile_mode = clone_repository_page.imp().profile_mode.take();

                                                clone_repository_page.imp().profile_mode.replace(profile_mode.clone());

                                                win2.save_repository(&mut new_repository, profile_mode);
                                                win2.imp().clone_repository_page.to_main_page();
                                                Continue(true)
                                            }
                                            Err(error) => {
                                                win2.imp().clone_repository_page.to_main_page();
                                                win2.show_error_dialog(&error);
                                                Continue(true)
                                            }
                                        }
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
                clone_repository_page: BagitCloneRepositoryPage,
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
                    let same_profile_name_number;

                    let app_database = win.imp().app_database.take();

                    match app_database.get_number_of_git_profiles_with_name(&profile_name, &profile_id.to_string()) {
                        Ok(number) => same_profile_name_number = number,
                        Err(error) => {
                            // TODO: Show error (maybe with a toast).

                            tracing::warn!("Could not get number of git profiles with name: {}", error);

                            return;
                        },
                    }

                    win.imp().app_database.replace(app_database);

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

                    let app_database = win.imp().app_database.take();

                    if !private_key_path.is_empty() {
                        win.save_ssh_passphrase(private_key_path.to_string(), passphrase.clone());
                    }

                    if let Err(error) = app_database.add_git_profile(&new_profile) {
                        tracing::warn!("Could not add Git profile: {}", error);

                        let toast = adw::Toast::new(&gettext("_Could not add Git profile"));
                        win.imp().toast_overlay.add_toast(toast);
                    }

                    win.imp().app_database.replace(app_database);

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

                                        let profile_mode = clone_repository_page.imp().profile_mode.take();

                                        clone_repository_page.imp().profile_mode.replace(profile_mode.clone());

                                        win2.save_repository(&mut new_repository, profile_mode);
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
                branch_name: &str,
                is_remote: bool,
                has_changed_files: bool
                | {
                    if has_changed_files {
                        win.show_checkout_dialog(branch_name.to_string(), is_remote);
                    } else {
                        win.imp().repository_page.checkout_branch_and_update_ui(branch_name.to_owned(), is_remote);
                    }
                }
            ),
        );

        self.imp().repository_page.connect_closure(
            "discard-dialog",
            false,
            closure_local!(@watch self as win => move |
                _repository_page: BagitRepositoryPage,
                is_discarding_folder: bool,
                discarded_element: &str
                | {
                win.show_discard_dialog(is_discarding_folder, discarded_element.to_string());
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

                    let gpg_passphrases = win.imp().gpg_passphrases.take();

                    if let Some(passphrase) = gpg_passphrases.get(&cloned_signing_key) {
                        repository_page.commit_files_and_update_ui(
                            &cloned_author,
                            &cloned_author_email,
                            &cloned_message,
                            &cloned_signing_key,
                            &passphrase,
                            &cloned_description,
                            cloned_need_to_save_profile,
                        );

                        win.imp().gpg_passphrases.replace(gpg_passphrases);

                        return;
                    }

                    win.imp().gpg_passphrases.replace(gpg_passphrases);

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
                                win2.save_gpg_passphrase(cloned_signing_key.clone(), passphrase.to_string().clone());

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
                action_type: ActionType,
                remote_branch_name: &str
                | {
                    let ctx: MainContext = glib::MainContext::default();
                    let cloned_username = String::from(username);
                    let cloned_private_key_path = String::from(private_key_path);
                    let cloned_remote_branch_name = String::from(remote_branch_name);

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
                                    action_type,
                                    cloned_remote_branch_name.clone()
                                );
                            }
                        ));

                        dialog.connect_closure("cancel", false, closure_local!(
                            @watch repository_page =>
                            move |
                            dialog: BagitSshActionDialog,
                            | {
                                dialog.close();
                                repository_page.try_to_find_correct_git_button_action();
                                repository_page.toggle_git_action_button(true);
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
                action_type: ActionType,
                remote_branch_name: &str
                | {
                    let ctx: MainContext = glib::MainContext::default();

                    let cloned_username = String::from(username);
                    let cloned_private_key_path= String::from(private_key_path);
                    let cloned_remote_branch_name = String::from(remote_branch_name);

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
                                if !private_key_path.is_empty() {
                                    win2.save_ssh_passphrase(private_key_path.to_string(), passphrase.to_string());
                                }

                                dialog.close();
                                repository_page.do_git_action_with_information(
                                    String::from(username),
                                    String::new(),
                                    String::from(private_key_path),
                                    String::from(passphrase),
                                    action_type,
                                    cloned_remote_branch_name.clone()
                                );
                            }
                        ));

                        dialog.connect_closure("cancel", false, closure_local!(
                            @watch repository_page =>
                            move |
                            dialog: BagitSshPassphraseDialog,
                            | {
                                dialog.close();
                                repository_page.try_to_find_correct_git_button_action();
                                repository_page.toggle_git_action_button(true);
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
                action_type: ActionType,
                remote_branch_name: &str
                | {
                    let ctx: MainContext = glib::MainContext::default();
                    let cloned_username = String::from(username);
                    let cloned_password = String::from(password);
                    let cloned_remote_branch_name = String::from(remote_branch_name);

                    ctx.spawn_local(clone!(
                        @weak win as win2 => async move {
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
                                    action_type,
                                    cloned_remote_branch_name.clone()
                                );
                            }
                        ));

                        dialog.connect_closure("cancel", false, closure_local!(
                            @watch repository_page =>
                            move |
                            dialog: BagitHttpsActionDialog,
                            | {
                                dialog.close();
                                repository_page.try_to_find_correct_git_button_action();
                                repository_page.toggle_git_action_button(true);
                            }
                        ));
                    }
                ));
            }),
        );

        self.imp().repository_page.connect_closure(
            "delete-branch",
            false,
            closure_local!(@watch self as win => move |
                _repository_page: BagitRepositoryPage,
                repository_path: &str,
                branch_name: &str,
                is_remote: bool
                | {
                    let ctx: MainContext = glib::MainContext::default();
                    let cloned_path = repository_path.to_string();
                    let cloned_branch_name = branch_name.to_string();

                    let body_text = format!("{} {}", gettext("_Delete branch confirmation"), branch_name);

                    ctx.spawn_local(clone!(@weak win as win2 => async move {
                        let delete_dialog = adw::MessageDialog::new(Some(&win2), Some(&gettext("_Delete branch")), Some(&body_text));
                        delete_dialog.add_response(&gettext("_Cancel"), &gettext("_Cancel"));
                        delete_dialog.add_response(&gettext("_Delete"), &gettext("_Delete"));
                        delete_dialog.set_close_response(&gettext("_Cancel"));
                        delete_dialog.set_response_appearance(&gettext("_Delete"), adw::ResponseAppearance::Destructive);
                        delete_dialog.present();
                        delete_dialog.connect_response(None, move |_dialog, response| {
                            if response == &gettext("_Delete") {
                                match Repository::open(&cloned_path) {
                                    Ok(repo) => {
                                        if is_remote {
                                            // When deleting a remote branch, we need to authenticate:
                                            win2.imp().repository_page.do_git_action_with_auth_check(ActionType::DeleteRemoteBranch, &cloned_branch_name);
                                        } else {
                                            match RepositoryUtils::delete_local_branch(&repo, &cloned_branch_name) {
                                                Ok(_) => {
                                                    win2.imp().repository_page.imp().branch_view.fetch_all_branches(cloned_path.clone());
                                                    win2.imp().repository_page.show_toast(&gettext("_Branch deleted"));
                                                },
                                                Err(error) => win2.show_error_dialog(&error.to_string())
                                            }
                                        }
                                    },
                                    Err(error) => win2.show_error_dialog(&error.to_string())
                                };
                            }
                        });
                    }));
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

    /// Used to show the dialog for discarding an element.
    pub fn show_discard_dialog(&self, is_discarding_folder: bool, discarded_element: String) {
        let (discard_message, discard_title) = if is_discarding_folder {
            (
                format!(
                    "{}\n{}",
                    gettext("_Discard folder message"),
                    discarded_element
                ),
                gettext("_Discard folder dialog"),
            )
        } else {
            (
                format!(
                    "{}\n{}",
                    gettext("_Discard file message"),
                    discarded_element
                ),
                gettext("_Discard file dialog"),
            )
        };

        let cloned_discared_element = discarded_element.clone();
        let ctx: MainContext = glib::MainContext::default();
        ctx.spawn_local(clone!(@weak self as win => async move {
            let discard_dialog = adw::MessageDialog::builder()
                .modal(true)
                .transient_for(&win)
                .heading(discard_title)
                .body(discard_message)
                .build();

            discard_dialog.add_response("cancel", &gettext("_Cancel"));
            discard_dialog.add_response("validate", &gettext("_Validate"));

            discard_dialog.connect_response(None,clone!(
                @weak win as win2,
                => move |_, response| {
                    match response {
                        "validate" => {
                            win2.imp().repository_page.discard_file_and_update_ui(&cloned_discared_element);
                        },
                        _ => {}
                    }
                }
            ));

            discard_dialog.present();
        }));
    }

    /// Saves a created repository.
    pub fn save_repository(&self, new_repository: &mut BagitRepository, profile_mode: ProfileMode) {
        self.add_list_row_to_all_repositories(&new_repository);
        let profile_id: Option<Uuid> = match profile_mode.get_profile_mode() {
            ProfileMode::SelectedProfile(profile) => Some(profile.profile_id),
            _ => None,
        };

        new_repository.git_profile_id = profile_id;

        let app_database = self.imp().app_database.take();

        if let Err(error) = app_database.add_repository(&new_repository) {
            tracing::warn!("Could not add repository: {}", error);

            let toast = adw::Toast::new(&gettext("_Could not add repository"));
            self.imp().toast_overlay.add_toast(toast);
        }

        self.imp().app_database.replace(app_database);

        self.update_recent_repositories();
        self.imp().stack.set_visible_child_name("main page");
    }
}
