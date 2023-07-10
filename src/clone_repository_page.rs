/* clone_repository_page.rs
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
use regex::Regex;

use crate::models::bagit_git_profile::BagitGitProfile;

mod imp {

    use uuid::Uuid;

    use super::*;

    // Object holding the state
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/clone-repository-page.ui")]
    pub struct BagitCloneRepositoryPage {
        #[template_child]
        pub back_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub url_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub location_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub clone_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub clone_button_and_profile: TemplateChild<gtk::Button>,
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
        pub git_profiles: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub profiles_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub passphrase_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub new_profile_revealer: TemplateChild<gtk::Revealer>,
    }

    #[template_callbacks]
    impl BagitCloneRepositoryPage {
        #[template_callback]
        fn go_back(&self, _back_button: &gtk::Button) {
            self.obj().emit_by_name::<()>("go-back", &[]);
        }

        #[template_callback]
        fn url_row_changed(&self, url_text_row: &adw::EntryRow) {
            self.clone_button
                .set_sensitive(self.obj().can_clone_button_be_sensitive());
            self.clone_button_and_profile
                .set_sensitive(self.obj().can_clone_button_with_new_profile_be_sensitive());

            self.passphrase_revealer
                .set_reveal_child(self.obj().is_using_ssh(&url_text_row.text()));
        }

        #[template_callback]
        fn location_row_changed(&self, _location_row: &adw::EntryRow) {
            self.clone_button
                .set_sensitive(self.obj().can_clone_button_be_sensitive());
            self.clone_button_and_profile
                .set_sensitive(self.obj().can_clone_button_with_new_profile_be_sensitive());
        }

        #[template_callback]
        fn profile_name_changed(&self, profile_name_row: &adw::EntryRow) {
            self.clone_button_and_profile
                .set_sensitive(self.obj().can_clone_button_with_new_profile_be_sensitive());

            self.obj().emit_by_name::<()>(
                "unique-name",
                &[
                    &self.profile_name_warning.clone(),
                    &profile_name_row.text().trim(),
                    &Uuid::new_v4().to_string(),
                ],
            );
        }

        #[template_callback]
        fn email_changed(&self, email_row: &adw::EntryRow) {
            if email_row.text().trim() == "" {
                self.email_error.set_visible(false);
                self.clone_button_and_profile
                    .set_sensitive(self.obj().can_clone_button_with_new_profile_be_sensitive());
            } else {
                // Check if the email is in a correct format :
                let is_email_correct_format = EmailAddress::is_valid(email_row.text().trim());
                self.email_error.set_visible(!is_email_correct_format);
                self.clone_button_and_profile.set_sensitive(
                    self.obj().can_clone_button_with_new_profile_be_sensitive()
                        && is_email_correct_format,
                );
            }
        }

        #[template_callback]
        fn row_clicked(&self, row: Option<adw::ActionRow>) {
            if row != None {
                let selected_row = row.unwrap();
                let index = selected_row.index();
                self.git_profiles.set_expanded(false);
                match index {
                    0 => {
                        self.stack.set_visible_child_name("simple clone");
                        self.new_profile_revealer.set_reveal_child(false);
                        self.git_profiles.set_title(&gettext("_No profile"));
                    }
                    1 => {
                        self.stack.set_visible_child_name("new profile");
                        self.new_profile_revealer.set_reveal_child(true);
                        self.git_profiles.set_title(&gettext("_New profile"));
                    }
                    _ => {
                        self.stack.set_visible_child_name("simple clone");
                        self.new_profile_revealer.set_reveal_child(false);
                        self.git_profiles.set_title(&selected_row.title());
                    }
                }
            }
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
        pub fn try_clone_repository(&self, _clone_button: &gtk::Button) {
            self.obj().emit_by_name::<()>(
                "clone-repository",
                &[&self.url_row.text(), &self.location_row.text()],
            );
        }

        #[template_callback]
        pub fn try_clone_repository_and_create_new_profile(&self, _clone_button: &gtk::Button) {
            self.obj().emit_by_name::<()>(
                "clone-repository-and-add-profile",
                &[
                    &self.url_row.text(),
                    &self.location_row.text(),
                    &self.profile_name.text(),
                    &self.email.text(),
                    &self.https_username.text(),
                    &self.https_pwd.text(),
                    &self.private_key_path.text(),
                ],
            );
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitCloneRepositoryPage {
        const NAME: &'static str = "BagitCloneRepositoryPage";
        type Type = super::BagitCloneRepositoryPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitCloneRepositoryPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("go-back").build(),
                    Signal::builder("select-location").build(),
                    Signal::builder("select-private-key").build(),
                    Signal::builder("unique-name")
                        .param_types([
                            gtk::Image::static_type(),
                            str::static_type(),
                            str::static_type(),
                        ])
                        .build(),
                    Signal::builder("clone-repository")
                        .param_types([str::static_type(), str::static_type()])
                        .build(),
                    Signal::builder("clone-repository-and-add-profile")
                        .param_types([
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
    impl WidgetImpl for BagitCloneRepositoryPage {}
    impl BoxImpl for BagitCloneRepositoryPage {}
}

glib::wrapper! {
    pub struct BagitCloneRepositoryPage(ObjectSubclass<imp::BagitCloneRepositoryPage>)
        @extends gtk::Widget,gtk::Window,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitCloneRepositoryPage {
    /**
     * Used to check if the clone button can be sensitive.
     */
    pub fn can_clone_button_be_sensitive(&self) -> bool {
        return (self.imp().url_row.text().trim() != "")
            && (self.imp().location_row.text().trim() != "");
    }

    /**
     * Used to check if the clone button (with new profile) can be sensitive.
     */
    pub fn can_clone_button_with_new_profile_be_sensitive(&self) -> bool {
        return (self.imp().url_row.text().trim() != "")
            && (self.imp().location_row.text().trim() != "")
            && (self.imp().profile_name.text().trim() != "");
    }

    /**
     * Check whether user is using https to clone a repository.
     */
    pub fn is_using_https(&self, url: &str) -> bool {
        let re = Regex::new(r"https://.*").unwrap();
        return re.is_match(url);
    }

    /**
     * Check whether user is using ssh to clone a repository.
     */
    pub fn is_using_ssh(&self, url: &str) -> bool {
        return url.contains("@");
    }

    /**
     * Used to clear page information.
     */
    pub fn clear_page(&self) {
        self.imp().url_row.set_text("");
        self.imp().location_row.set_text("");
        self.imp().profile_name.set_text("");
        self.imp().email.set_text("");
        self.imp().https_username.set_text("");
        self.imp().https_pwd.set_text("");
        self.imp().private_key_path.set_text("");
        self.imp().passphrase.set_text("");

        self.clear_profiles_list();

        self.imp().profile_name_warning.set_visible(false);
        self.imp().email_error.set_visible(false);

        self.imp().new_profile_revealer.set_reveal_child(false);
        self.imp().passphrase_revealer.set_reveal_child(false);
        self.imp().stack.set_visible_child_name("simple clone");
    }

    /**
     * Use to clear profiles list.
     */
    pub fn clear_profiles_list(&self) {
        let mut row = self.imp().profiles_list.row_at_index(2);
        while row != None {
            self.imp().profiles_list.remove(&row.unwrap());
            row = self.imp().profiles_list.row_at_index(2);
        }

        // We make sure that the selected row is the default one :
        self.imp().profiles_list.unselect_all();
        self.imp()
            .profiles_list
            .select_row(self.imp().profiles_list.row_at_index(0).as_ref());
        self.imp().git_profiles.set_title(&gettext("_No profile"));
    }

    /**
     * Used to add a new git profile row to the profiles list.
     */
    pub fn add_git_profile_row(&self, profile: BagitGitProfile) {
        let action_row = adw::ActionRow::new();
        action_row.set_title(&profile.profile_name);

        self.imp().profiles_list.append(&action_row);
    }
}
