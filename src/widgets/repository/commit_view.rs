/* commit_view.rs
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

use adw::prelude::{EditableExt, WidgetExt};
use adw::subclass::prelude::*;
use adw::traits::{ExpanderRowExt, PreferencesRowExt};
use email_address::EmailAddress;
use gettextrs::gettext;
use gtk::glib::subclass::Signal;
use gtk::prelude::ObjectExt;
use gtk::subclass::widget::CompositeTemplateInitializingExt;
use gtk::template_callbacks;
use gtk::{glib, prelude::*, CompositeTemplate};
use once_cell::sync::Lazy;
use std::cell::RefCell;

use crate::models::bagit_git_profile::BagitGitProfile;
use crate::utils::commit_view_profile_mode_type::{
    CommitViewProfileModeType, CommitViewProfileModeValues,
};
use crate::utils::git_profile_utils::GitProfileUtils;
use crate::utils::profile_mode::ProfileMode;

mod imp {

    use gtk::gio::Settings;

    use crate::{utils::db::AppDatabase, widgets::profile_dialog::BagitProfileDialog};

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(
        resource = "/com/skilldary/bagit/desktop/ui/widgets/repository/bagit-commit-view.ui"
    )]
    pub struct BagitCommitView {
        #[template_child]
        pub selected_profile_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub file_information_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub git_profiles: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub profiles_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub no_profile_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub author_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub author_email_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub email_error: TemplateChild<gtk::Image>,
        #[template_child]
        pub message_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub message_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub description_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub signing_key_row: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub save_profile_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub commit_button: TemplateChild<gtk::Button>,

        pub profile_mode: RefCell<ProfileMode>,

        pub app_database: AppDatabase,
    }

    #[template_callbacks]
    impl BagitCommitView {
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
        fn row_clicked(&self, row: Option<adw::ActionRow>) {
            if row != None {
                let selected_row = row.unwrap();
                let index = selected_row.index();
                let profile_title = selected_row.title().to_string();
                self.git_profiles.set_expanded(false);
                self.obj().set_profile_mode(index, profile_title);
            }
        }
        #[template_callback]
        fn author_row_changed(&self, _author_row: &adw::EntryRow) {
            self.obj().emit_by_name::<()>("toggle-commit-button", &[]);
        }
        #[template_callback]
        fn author_email_changed(&self, email_row: &adw::EntryRow) {
            self.obj().emit_by_name::<()>("toggle-commit-button", &[]);

            // Check if we need to show the error image :
            self.email_error
                .set_visible(!EmailAddress::is_valid(&email_row.text()));
        }
        #[template_callback]
        fn message_row_changed(&self, message_row: &adw::EntryRow) {
            self.message_revealer
                .set_reveal_child(message_row.text().trim().len() > 50);
            self.obj().emit_by_name::<()>("toggle-commit-button", &[]);
        }
        #[template_callback]
        fn save_profile_button_changed(&self, check_button: &gtk::CheckButton) {
            let settings = Settings::new("com.skilldary.bagit.desktop");
            let state = check_button.is_active();

            settings
                .set_boolean("is-saving-commit-profile-enabled", state)
                .expect("Could not set setting.");
        }
        #[template_callback]
        fn commit_files(&self, _commit_button: &gtk::Button) {
            let profile_mode = self.profile_mode.take();
            self.profile_mode.replace(profile_mode.clone());

            match profile_mode {
                ProfileMode::SelectedProfile(profile) => {
                    self.obj().emit_by_name::<()>(
                        "commit-files",
                        &[
                            &profile.username,
                            &profile.email,
                            &self.message_row.text().trim(),
                            &profile.signing_key,
                            &self.description_row.text().trim(),
                            &false,
                        ],
                    );
                }
                _ => {
                    self.obj().emit_by_name::<()>(
                        "commit-files",
                        &[
                            &self.author_row.text().trim(),
                            &self.author_email_row.text().trim(),
                            &self.message_row.text().trim(),
                            &self.signing_key_row.text().trim(),
                            &self.description_row.text().trim(),
                            &self.obj().does_profile_needs_to_be_saved(),
                        ],
                    );
                }
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitCommitView {
        const NAME: &'static str = "BagitCommitView";
        type Type = super::BagitCommitView;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for BagitCommitView {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("commit-files")
                        .param_types([
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            bool::static_type(),
                        ])
                        .build(),
                    Signal::builder("toggle-commit-button").build(),
                    Signal::builder("select-profile")
                        .param_types([str::static_type()])
                        .build(),
                    Signal::builder("remove-profile").build(),
                    Signal::builder("update-git-config").build(),
                ]
            });
            SIGNALS.as_ref()
        }
        fn constructed(&self) {
            self.parent_constructed();

            let settings = Settings::new("com.skilldary.bagit.desktop");

            self.save_profile_button
                .set_active(settings.boolean("is-saving-commit-profile-enabled"));
        }
    }
    impl WidgetImpl for BagitCommitView {}
    impl BoxImpl for BagitCommitView {}
}
glib::wrapper! {
    pub struct BagitCommitView(ObjectSubclass<imp::BagitCommitView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitCommitView {
    /// Used to define the profile mode used for committing.
    pub fn set_profile_mode(&self, index: i32, profile_title: String) {
        match index {
            CommitViewProfileModeType::NO_PROFILE => {
                self.show_no_selected_profile();
                self.imp().obj().emit_by_name::<()>("remove-profile", &[]);
            }
            _ => {
                self.imp()
                    .obj()
                    .emit_by_name::<()>("select-profile", &[&profile_title]);
            }
        }
    }

    /// Used to initialize the commit view.
    pub fn init_commit_view(&self) {
        self.clear_fields();
        self.clear_profiles_list(false);
        self.update_commit_view(0);

        let all_profiles = self.imp().app_database.get_all_git_profiles();
        for profile in all_profiles {
            self.add_git_profile_row(&profile);
        }

        self.imp().git_profiles.set_expanded(false);
    }

    /// Used to clear the selected profile and all the fields.
    pub fn clear_fields(&self) {
        self.imp().author_row.set_text("");
        self.imp().author_email_row.set_text("");
        self.imp().message_row.set_text("");
        self.imp().signing_key_row.set_text("");
    }

    ///Used to update the label of files selected and the commit button of the commit view.
    pub fn update_commit_view(&self, total_selected_files: i32) {
        let new_text = if total_selected_files == 0 {
            gettext("_No committed file")
        } else if total_selected_files == 1 {
            format!("{} {}", total_selected_files, gettext("_Committed file"))
        } else {
            format!("{} {}", total_selected_files, gettext("_Committed files"))
        };
        self.imp().file_information_label.set_text(&new_text);

        self.imp()
            .commit_button
            .set_sensitive(self.can_commit_button_be_activated(total_selected_files));
    }

    /// Used to define if the necessary fields for a commit are completed.
    pub fn can_commit_button_be_activated(&self, total_selected_files: i32) -> bool {
        let is_author_text_filled = !self.imp().author_row.text().trim().is_empty();
        let is_author_mail_filled = !self.imp().author_email_row.text().trim().is_empty();
        let is_message_text_filled = !self.imp().message_row.text().trim().is_empty();
        let is_author_mail_valid = EmailAddress::is_valid(&self.imp().author_email_row.text());

        return match self.imp().profile_mode.borrow().get_profile_mode() {
            ProfileMode::SelectedProfile(_) => {
                is_message_text_filled && (total_selected_files != 0)
            }
            _ => {
                is_author_mail_filled
                    && is_message_text_filled
                    && is_author_text_filled
                    && (total_selected_files != 0)
                    && is_author_mail_valid
            }
        };
    }

    /**
     * Use to clear profiles list.
     */
    pub fn clear_profiles_list(&self, unselect_all: bool) {
        let mut row = self.imp().profiles_list.row_at_index(1);
        while row != None {
            self.imp().profiles_list.remove(&row.unwrap());
            row = self.imp().profiles_list.row_at_index(1);
        }

        // We make sure that the selected row is the default one :
        if unselect_all {
            self.imp().profiles_list.unselect_all();
        }
    }

    /// Used to set and show the selected profile.
    pub fn set_and_show_selected_profile(&self, selected_profile: BagitGitProfile) {
        self.imp()
            .profile_mode
            .replace(ProfileMode::SelectedProfile(selected_profile.clone()));

        self.imp().no_profile_revealer.set_reveal_child(false);
        self.imp().no_profile_revealer.set_visible(false);
        self.imp().selected_profile_revealer.set_visible(true);
        self.imp().selected_profile_revealer.set_reveal_child(true);
        self.imp()
            .git_profiles
            .set_title(&selected_profile.profile_name);
    }

    /// Used to show no selected profile.
    pub fn show_no_selected_profile(&self) {
        self.imp().profile_mode.replace(ProfileMode::NoProfile);
        self.imp().no_profile_revealer.set_visible(true);
        self.imp().no_profile_revealer.set_reveal_child(true);
        self.imp().selected_profile_revealer.set_reveal_child(false);
        self.imp().selected_profile_revealer.set_visible(false);
        self.imp().git_profiles.set_title(&gettext("_No profile"));
    }

    /**
     * Used to add a new git profile row to the profiles list.
     */
    pub fn add_git_profile_row(&self, profile: &BagitGitProfile) {
        let action_row = GitProfileUtils::build_profile_row(profile);

        self.imp().profiles_list.append(&action_row);
    }

    /**
     * Used to update the git profiles list.
     * This will do the following :
     * - Update the list (remove, add entries)
     * - Check if the current selected profile si still valid. If not, will select no profile.
     */
    pub fn update_git_profiles_list(&self) {
        let profile_mode = self.imp().profile_mode.take();
        let new_list = self.imp().app_database.get_all_git_profiles();

        self.clear_profiles_list(false);

        for profile in &new_list {
            self.add_git_profile_row(profile);
        }

        // If we had selected a profile, we check that this profile still exist.
        match profile_mode {
            ProfileMode::SelectedProfile(selected_profile) => {
                let mut found_profile: Option<BagitGitProfile> = None;
                for profile in &new_list {
                    if profile.profile_id == selected_profile.profile_id {
                        found_profile = Some(profile.clone());
                        break;
                    }
                }
                // If we found a profile, the selected one still exist and we eventually update it's name if it has changed :
                match found_profile {
                    Some(profile) => {
                        self.set_and_show_selected_profile(profile);
                        self.emit_by_name::<()>("update-git-config", &[]);
                    }
                    None => {
                        // Else, we go back to the No profile state.
                        self.set_profile_mode(CommitViewProfileModeType::NO_PROFILE, "".to_string())
                    }
                }
            }
            _ => {}
        }
    }

    /// Used to know if a new profile needs to be saved
    pub fn does_profile_needs_to_be_saved(&self) -> bool {
        let is_using_no_profile_mode = match self.imp().profile_mode.borrow().get_profile_mode() {
            ProfileMode::NoProfile => true,
            _ => false,
        };

        let is_save_button_activated = self.imp().save_profile_button.is_active();

        return is_save_button_activated && is_using_no_profile_mode;
    }
}
