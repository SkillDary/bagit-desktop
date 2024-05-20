/* repository_config_view.rs
 *
 * Copyright 2024 SkillDary
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
use crate::utils::commit_view_profile_mode_type::{
    CommitViewProfileModeType, CommitViewProfileModeValues,
};
use crate::utils::git_profile_utils::GitProfileUtils;
use crate::utils::profile_mode::ProfileMode;
use crate::{utils::db::AppDatabase, widgets::profile_dialog::BagitProfileDialog};
use adw::prelude::EditableExt;
use adw::subclass::prelude::*;
use adw::traits::{ExpanderRowExt, PreferencesRowExt};
use gettextrs::gettext;
use gtk::glib::subclass::Signal;
use gtk::subclass::widget::CompositeTemplateInitializingExt;
use gtk::template_callbacks;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;
use std::cell::Cell;
use std::cell::RefCell;

use gtk::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(
        resource = "/com/skilldary/bagit/desktop/ui/widgets/repository/bagit-repository-config-view.ui"
    )]
    pub struct BagitRepositoryConfigView {
        #[template_child]
        pub repo_url_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub profiles_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub selected_profile_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub git_profiles: TemplateChild<adw::ExpanderRow>,

        pub is_initializing_remote_origin: Cell<bool>,
        pub app_database: RefCell<AppDatabase>,
        pub profile_mode: RefCell<ProfileMode>,
    }

    #[template_callbacks]
    impl BagitRepositoryConfigView {
        #[template_callback]
        fn remote_url_changed(&self, repo_url_row: &adw::EntryRow) {
            if self.is_initializing_remote_origin.get() {
                self.is_initializing_remote_origin.set(false);
                return;
            }

            let new_url = repo_url_row.text();
            self.obj()
                .emit_by_name::<()>("remote-url-changed", &[&new_url.trim()]);
        }

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
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitRepositoryConfigView {
        const NAME: &'static str = "BagitRepositoryConfigView";
        type Type = super::BagitRepositoryConfigView;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for BagitRepositoryConfigView {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("remote-url-changed")
                        .param_types([str::static_type()])
                        .build(),
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
            self.is_initializing_remote_origin.set(false);

            let mut app_database = self.app_database.take();
            app_database.create_connection();
            self.app_database.replace(app_database);
        }
    }
    impl WidgetImpl for BagitRepositoryConfigView {}
    impl BoxImpl for BagitRepositoryConfigView {}
}
glib::wrapper! {
    pub struct BagitRepositoryConfigView(ObjectSubclass<imp::BagitRepositoryConfigView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitRepositoryConfigView {
    /// Initializes the repository's remote url.
    pub fn set_remote_url(&self, remote_url: &str) {
        self.imp().is_initializing_remote_origin.set(true);
        self.imp().repo_url_row.set_text(remote_url);
    }

    /// Sets the profile mode used to commit.
    pub fn set_profile_mode(&self, index: i32, profile_title: String) {
        match index {
            CommitViewProfileModeType::NO_PROFILE => {
                self.imp().obj().emit_by_name::<()>("remove-profile", &[]);
            }
            _ => {
                self.imp()
                    .obj()
                    .emit_by_name::<()>("select-profile", &[&profile_title]);
            }
        }
    }

    /// Shows no selected profile.
    pub fn show_no_selected_profile(&self) {
        self.imp().profile_mode.replace(ProfileMode::NoProfile);
        self.imp().selected_profile_revealer.set_reveal_child(false);
        self.imp().selected_profile_revealer.set_visible(false);
        self.imp().git_profiles.set_title(&gettext("_No profile"));
    }

    /// Clears the profile list.
    fn clear_profiles_list(&self, unselect_all: bool) {
        let mut row = self.imp().profiles_list.row_at_index(1);
        while row != None {
            self.imp().profiles_list.remove(&row.unwrap());
            row = self.imp().profiles_list.row_at_index(1);
        }

        // We make sure that the selected row is the default one:
        if unselect_all {
            self.imp().profiles_list.unselect_all();
        }
    }

    /// Sets and shows the selected profile.
    pub fn set_and_show_selected_profile(&self, selected_profile: BagitGitProfile) {
        self.imp()
            .profile_mode
            .replace(ProfileMode::SelectedProfile(selected_profile.clone()));

        self.imp().selected_profile_revealer.set_visible(true);
        self.imp().selected_profile_revealer.set_reveal_child(true);
        self.imp()
            .git_profiles
            .set_title(&selected_profile.profile_name);
    }

    /// Adds a new git profile row to the profile list.
    fn add_git_profile_row(&self, profile: &BagitGitProfile) {
        let action_row = GitProfileUtils::build_profile_row(profile);

        self.imp().profiles_list.append(&action_row);
    }

    /// Builds the profile list to be shown and returns the list of profiles.
    pub fn build_profiles_list(&self) -> Vec<BagitGitProfile> {
        let all_profiles;

        let app_database = self.imp().app_database.take();

        match app_database.get_all_git_profiles() {
            Ok(profiles) => all_profiles = profiles,
            Err(error) => {
                // TODO: Show error (maybe with a toast).

                tracing::warn!("Could not get all Git profiles: {}", error);

                return vec![];
            }
        }

        self.imp().app_database.replace(app_database);

        self.clear_profiles_list(false);
        for profile in &all_profiles {
            self.add_git_profile_row(&profile);
        }

        all_profiles
    }

    /// Updates the git profile list.
    ///
    /// This will do the following:
    ///   - Update the list (remove, add entries)
    ///   - Check if the current selected profile is still valid. Otherwise, it will select no profile.
    pub fn update_git_profiles_list(&self) {
        let all_profiles = self.build_profiles_list();

        let profile_mode = self.imp().profile_mode.take();

        // If we had selected a profile, we check that this profile still exist.
        match profile_mode {
            ProfileMode::SelectedProfile(selected_profile) => {
                let mut found_profile: Option<BagitGitProfile> = None;

                for profile in all_profiles {
                    if profile.profile_id == selected_profile.profile_id {
                        found_profile = Some(profile.clone());
                        break;
                    }
                }

                // If we found a profile, the selected one still exist and we eventually update it's name if it has changed:
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

    /// Retrieves the current ProfileMode of the repository.
    pub fn get_profile_mode(&self) -> ProfileMode {
        let profile_mode = self.imp().profile_mode.take();

        self.imp().profile_mode.replace(profile_mode.clone());

        return profile_mode;
    }
}
