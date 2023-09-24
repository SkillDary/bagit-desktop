/* create_repository_page.rs
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

use adw::subclass::prelude::*;
use adw::traits::ExpanderRowExt;
use adw::traits::PreferencesRowExt;
use email_address::EmailAddress;
use gettextrs::gettext;
use gtk::glib::subclass::Signal;
use gtk::{glib, prelude::*, template_callbacks, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::models::bagit_git_profile::BagitGitProfile;
use crate::utils::create_page_profile_mode_type::CreatePageProfileModeType;
use crate::utils::create_page_profile_mode_type::CreatePageProfileModeValues;
use crate::utils::git_profile_utils::GitProfileUtils;
use std::cell::RefCell;

use uuid::Uuid;

use crate::utils::profile_mode::ProfileMode;

mod imp {

    use crate::{utils::db::AppDatabase, widgets::profile_dialog::BagitProfileDialog};

    use super::*;

    // Object holding the state
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/create-repository-page.ui")]
    pub struct BagitCreateRepositoryPage {
        #[template_child]
        pub selected_profile_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub repository_name_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub location_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub create_repository_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub create_repository_button_and_profile: TemplateChild<gtk::Button>,
        #[template_child]
        pub new_git_profile: TemplateChild<gtk::Box>,
        #[template_child]
        pub profile_name: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub profile_name_warning: TemplateChild<gtk::Image>,
        #[template_child]
        pub email: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub email_error: TemplateChild<gtk::Image>,
        #[template_child]
        pub https_username: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub https_pwd: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub private_key_path: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub passphrase: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub signing_key: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub git_profiles: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub profiles_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub button_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub passphrase_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub new_profile_revealer: TemplateChild<gtk::Revealer>,

        pub profile_mode: RefCell<ProfileMode>,

        pub app_database: AppDatabase,
    }

    #[template_callbacks]
    impl BagitCreateRepositoryPage {
        #[template_callback]
        fn show_profile_information(&self, _button: &gtk::Button) {
            let profile_mode = self.profile_mode.borrow().get_profile_mode();

            match profile_mode {
                ProfileMode::SelectedProfile(profile) => {
                    let profile_dialog = BagitProfileDialog::new(&profile);
                    profile_dialog.set_modal(true);
                    profile_dialog.present();
                }
                _ => {}
            };
        }

        #[template_callback]
        fn go_back(&self, _back_button: &gtk::Button) {
            self.obj().emit_by_name::<()>("go-back", &[]);
        }

        #[template_callback]
        fn repository_name_row_changed(&self, _repository_name_text_row: &adw::EntryRow) {
            self.create_repository_button
                .set_sensitive(self.obj().can_create_button_be_sensitive());

            self.create_repository_button_and_profile
                .set_sensitive(self.obj().can_create_button_with_new_profile_be_sensitive());
        }

        #[template_callback]
        fn location_row_changed(&self, _location_row: &adw::EntryRow) {
            self.create_repository_button
                .set_sensitive(self.obj().can_create_button_be_sensitive());
            self.create_repository_button_and_profile
                .set_sensitive(self.obj().can_create_button_with_new_profile_be_sensitive());
        }

        #[template_callback]
        fn profile_name_changed(&self, profile_name_row: &adw::EntryRow) {
            self.create_repository_button_and_profile
                .set_sensitive(self.obj().can_create_button_with_new_profile_be_sensitive());

            // We check if the name of the profile is unique:
            let same_profile_name_number;

            match self.app_database.get_number_of_git_profiles_with_name(
                &profile_name_row.text().trim(),
                &Uuid::new_v4().to_string(),
            ) {
                Ok(number) => same_profile_name_number = number,
                Err(error) => {
                    // TODO: Show error (maybe with a toast).

                    tracing::warn!("Could not get number of git profiles with name: {}", error);

                    return;
                }
            }

            self.profile_name_warning
                .clone()
                .set_visible(same_profile_name_number != 0);
        }

        #[template_callback]
        fn email_changed(&self, email_row: &adw::EntryRow) {
            if email_row.text().trim() == "" {
                self.email_error.set_visible(false);
                self.create_repository_button_and_profile
                    .set_sensitive(self.obj().can_create_button_with_new_profile_be_sensitive());
            } else {
                // Check if the email is in a correct format:
                let is_email_correct_format = EmailAddress::is_valid(email_row.text().trim());
                self.email_error.set_visible(!is_email_correct_format);
                self.create_repository_button_and_profile.set_sensitive(
                    self.obj().can_create_button_with_new_profile_be_sensitive()
                        && is_email_correct_format,
                );
            }
        }

        #[template_callback]
        fn row_clicked(&self, row: Option<adw::ActionRow>) {
            if row == None {
                return;
            }

            let selected_row = row.unwrap();
            let index = selected_row.index();
            let profile_title = selected_row.title().to_string();
            self.git_profiles.set_expanded(false);

            self.obj().set_profile_mode(index, profile_title);
        }

        #[template_callback]
        fn select_location(&self, _select_location_button: &gtk::Button) {
            self.obj().emit_by_name::<()>("select-location", &[]);
        }

        #[template_callback]
        fn select_private_key_path(&self, _private_key_button: &gtk::Button) {
            self.obj().emit_by_name::<()>("select-private-key", &[]);
        }

        #[template_callback]
        pub fn create_repository(&self, _create_button: &gtk::Button) {
            self.obj().emit_by_name::<()>(
                "create-repository",
                &[&self.repository_name_row.text(), &self.location_row.text()],
            );
        }

        #[template_callback]
        pub fn create_repository_and_create_new_profile(&self, _create_button: &gtk::Button) {
            self.obj().emit_by_name::<()>(
                "create-repository-and-add-profile",
                &[
                    &self.repository_name_row.text(),
                    &self.location_row.text(),
                    &self.profile_name.text(),
                    &self.email.text(),
                    &self.https_username.text(),
                    &self.https_pwd.text(),
                    &self.private_key_path.text(),
                    &self.signing_key.text(),
                ],
            );
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitCreateRepositoryPage {
        const NAME: &'static str = "BagitCreateRepositoryPage";
        type Type = super::BagitCreateRepositoryPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitCreateRepositoryPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("go-back").build(),
                    Signal::builder("select-location").build(),
                    Signal::builder("select-private-key").build(),
                    Signal::builder("create-repository")
                        .param_types([str::static_type(), str::static_type()])
                        .build(),
                    Signal::builder("create-repository-and-add-profile")
                        .param_types([
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                        ])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitCreateRepositoryPage {}
    impl BoxImpl for BagitCreateRepositoryPage {}
}

glib::wrapper! {
    pub struct BagitCreateRepositoryPage(ObjectSubclass<imp::BagitCreateRepositoryPage>)
        @extends gtk::Widget,gtk::Window,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitCreateRepositoryPage {
    /// Shows the selected profile.
    pub fn show_selected_profile(&self, profile_name: &str) {
        self.imp().selected_profile_revealer.set_visible(true);
        self.imp().selected_profile_revealer.set_reveal_child(true);
        self.imp().git_profiles.set_title(profile_name);
    }

    /// Shows that there is no selected profile.
    pub fn show_no_selected_profile(&self, new_state_title: &str) {
        self.imp().selected_profile_revealer.set_reveal_child(false);
        self.imp().selected_profile_revealer.set_visible(false);
        self.imp().git_profiles.set_title(new_state_title);
    }

    /// Checks if the create button can be sensitive.
    pub fn can_create_button_be_sensitive(&self) -> bool {
        return (self.imp().repository_name_row.text().trim() != "")
            && (self.imp().location_row.text().trim() != "");
    }

    /// Checks if the create button (with new profile) can be sensitive.
    pub fn can_create_button_with_new_profile_be_sensitive(&self) -> bool {
        return (self.imp().repository_name_row.text().trim() != "")
            && (self.imp().location_row.text().trim() != "")
            && (self.imp().profile_name.text().trim() != "")
            && (self.imp().https_username.text().trim() != "")
            && (EmailAddress::is_valid(self.imp().email.text().trim())
                && self.imp().email.text().trim() != "");
    }

    /// Clears page information.
    pub fn clear_page(&self) {
        self.imp().repository_name_row.set_text("");
        self.imp().location_row.set_text("");
        self.imp().profile_name.set_text("");
        self.imp().email.set_text("");
        self.imp().https_username.set_text("");
        self.imp().https_pwd.set_text("");
        self.imp().private_key_path.set_text("");
        self.imp().passphrase.set_text("");
        self.imp().signing_key.set_text("");

        // We clear the list informations and the profile mode used:
        self.clear_profiles_list(true);
        self.show_no_selected_profile(&gettext("_No profile"));
        self.imp().profile_mode.take();

        self.imp().profile_name_warning.set_visible(false);
        self.imp().email_error.set_visible(false);

        self.imp().new_profile_revealer.set_reveal_child(false);
        self.imp().passphrase_revealer.set_reveal_child(false);
        self.imp()
            .button_stack
            .set_visible_child_name("simple create");
    }

    /// Updates the list of git profiles.
    ///
    /// This will do the following:
    ///   - Update the list (remove, add entries)
    ///   - Check if the current selected profile is still valid. If not, will select no profile.
    pub fn update_git_profiles_list(&self, new_list: &Vec<BagitGitProfile>) {
        let profile_mode = self.imp().profile_mode.take();

        self.clear_profiles_list(false);

        for profile in new_list {
            self.add_git_profile_row(&profile);
        }

        // If we had selected a profile, we check that this profile still exist.
        match profile_mode {
            ProfileMode::SelectedProfile(selected_profile) => {
                let mut found_profile: Option<BagitGitProfile> = None;
                for profile in new_list {
                    if profile.profile_id == selected_profile.profile_id {
                        found_profile = Some(profile.clone());
                        break;
                    }
                }
                // If we have found a profile, the selected one still exist, and we eventually update its name if it has changed.
                if found_profile.is_some() {
                    self.set_profile_mode(3, found_profile.unwrap().profile_name)
                } else {
                    // Otherwise, we return to the No profile state.
                    self.set_profile_mode(3, "".to_string())
                }
            }
            _ => {}
        }
    }

    /// Sets the profile mode used for creating a repository.
    pub fn set_profile_mode(&self, index: i32, profile_title: String) {
        match index {
            CreatePageProfileModeType::NO_PROFILE => {
                self.imp()
                    .button_stack
                    .set_visible_child_name("simple create");
                self.imp().new_profile_revealer.set_reveal_child(false);
                self.imp().profile_mode.replace(ProfileMode::NoProfile);
                self.show_no_selected_profile(&gettext("_No profile"));
            }
            CreatePageProfileModeType::NEW_PROFILE => {
                self.imp()
                    .button_stack
                    .set_visible_child_name("new profile");
                self.imp().new_profile_revealer.set_reveal_child(true);
                self.imp().profile_mode.replace(ProfileMode::NewProfile);
                self.show_no_selected_profile(&gettext("_New profile"));
            }
            _ => {
                self.imp()
                    .button_stack
                    .set_visible_child_name("simple create");

                let found_profile;

                match self
                    .imp()
                    .app_database
                    .get_git_profile_from_name(&profile_title)
                {
                    Ok(profile) => found_profile = profile,
                    Err(error) => {
                        // TODO: Show error (maybe with a toast).

                        tracing::warn!("Could not get Git profile from name: {}", error);

                        return;
                    }
                }

                match found_profile {
                    Some(profile) => {
                        self.show_selected_profile(&profile.profile_name);
                        self.imp()
                            .profile_mode
                            .replace(ProfileMode::SelectedProfile(profile));
                    }
                    None => {}
                }
                self.imp().new_profile_revealer.set_reveal_child(false);
            }
        }
    }

    /// Clears the profile list.
    pub fn clear_profiles_list(&self, unselect_all: bool) {
        let mut row = self.imp().profiles_list.row_at_index(2);
        while row != None {
            self.imp().profiles_list.remove(&row.unwrap());
            row = self.imp().profiles_list.row_at_index(2);
        }

        // We make sure that the selected row is the default one :
        if unselect_all {
            self.imp().profiles_list.unselect_all();
        }
    }

    /// Adds a new git profile row to the list of profiles.
    pub fn add_git_profile_row(&self, profile: &BagitGitProfile) {
        let action_row = GitProfileUtils::build_profile_row(&profile);

        self.imp().profiles_list.append(&action_row);
    }
}
