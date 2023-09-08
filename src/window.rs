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
        self, action_type::ActionType, profile_mode::ProfileMode,
        repository_utils::RepositoryUtils, selected_repository::SelectedRepository,
    },
    widgets::https_action_dialog::BagitHttpsActionDialog,
    widgets::passphrase_dialog::BagitGpgPassphraseDialog,
    widgets::{
        ssh_action_dialog::BagitSshActionDialog, ssh_passphrase_dialog::BagitSshPassphraseDialog,
    },
};
use adw::{
    subclass::prelude::*,
    traits::{ActionRowExt, PreferencesRowExt},
};
use gettextrs::gettext;
use git2::Repository;
use gtk::{
    gio,
    glib::{self, MainContext, Priority},
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
                        "repository page" => {
                            win.repository_page.update_commits_sidebar();
                            win.repository_page.imp().commit_view.update_git_profiles_list()
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

                    if found_repository.is_some() {
                        let repo = found_repository.unwrap();

                        match SelectedRepository::try_fetching_selected_repository(&repo) {
                            Ok(selected_repository) => {
                                win.imp().repository_page.init_repository_page(selected_repository);
                                win.imp().stack.set_visible_child_name("repository page");
                            }
                            Err(_) => {}
                        }
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
                                let folder_name = RepositoryUtils::get_folder_name_from_os(folder_path_str);

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

    /// Used to initialize the repositories.
    fn init_repositories(&self) {
        let all_repositories: Vec<BagitRepository> = self.imp().app_database.get_all_repositories();

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
                let git_profiles: Vec<BagitGitProfile> = win.imp().app_database.get_all_git_profiles();
                for profile in git_profiles {
                    win.imp().clone_repository_page.add_git_profile_row(&profile);
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
                                        win2.save_repository(&elements.0, &elements.1);
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
                                        win2.save_repository(&elements.0, &elements.1);
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

    pub fn save_repository(&self, repository_name: &str, repository_path: &str) {
        self.add_list_row(&repository_name, &repository_path);

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

        self.imp()
            .app_database
            .add_repository(&repository_name, &repository_path, profile_id);
        self.imp().stack.set_visible_child_name("main page");
    }
}
