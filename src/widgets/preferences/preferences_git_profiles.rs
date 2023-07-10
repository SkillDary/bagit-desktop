/* preferences_git_profiles.rs
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
use adw::traits::{ActionRowExt, EntryRowExt, PreferencesRowExt};
use email_address::EmailAddress;
use gettextrs::gettext;
use gtk::glib::subclass::Signal;
use gtk::template_callbacks;
use gtk::traits::{BoxExt, ButtonExt, EditableExt, WidgetExt};
use gtk::{glib, prelude::*, CompositeTemplate};
use once_cell::sync::Lazy;
use uuid::Uuid;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(
        resource = "/com/skilldary/bagit/desktop/ui/widgets/preferences/bagit-preferences-git-profiles.ui"
    )]
    pub struct BagitPreferencesGitProfiles {
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub git_profiles: TemplateChild<gtk::ListBox>,

        pub all_profiles: Vec<i32>,
    }

    #[template_callbacks]
    impl BagitPreferencesGitProfiles {
        #[template_callback]
        fn delete_profil(&self, _button: gtk::Button) {}

        #[template_callback]
        fn add_expander_row(&self, _button: gtk::Button) {
            self.obj().emit_by_name::<()>("can-add-profile", &[]);
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitPreferencesGitProfiles {
        const NAME: &'static str = "BagitPreferencesGitProfiles";
        type Type = super::BagitPreferencesGitProfiles;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitPreferencesGitProfiles {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("save-profile")
                        .param_types([
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            gtk::Label::static_type(),
                            adw::EntryRow::static_type(),
                        ])
                        .build(),
                    Signal::builder("delete-profile")
                        .param_types([adw::ExpanderRow::static_type(), str::static_type()])
                        .build(),
                    Signal::builder("select-location")
                        .param_types([adw::EntryRow::static_type()])
                        .build(),
                    Signal::builder("can-add-profile").build(),
                    Signal::builder("profile-modified")
                        .param_types([
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            str::static_type(),
                            gtk::Revealer::static_type(),
                        ])
                        .build(),
                    Signal::builder("unique-name")
                        .param_types([
                            gtk::Image::static_type(),
                            str::static_type(),
                            str::static_type(),
                        ])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitPreferencesGitProfiles {}
    impl BoxImpl for BagitPreferencesGitProfiles {}
}

glib::wrapper! {
    pub struct BagitPreferencesGitProfiles(ObjectSubclass<imp::BagitPreferencesGitProfiles>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitPreferencesGitProfiles {
    /**
     * Used to create a new profile row.
     */
    pub fn add_new_git_profile(
        &self,
        profile_id: Uuid,
        profile_name: &str,
        email: &str,
        username: &str,
        password: &str,
        path: &str,
        is_expanded: bool,
    ) {
        let expander_row = adw::ExpanderRow::new();
        let profile_title = gtk::Label::new(Some(&""));
        if profile_name == "" {
            profile_title.add_css_class("dim-label");
            profile_title.set_text(&gettext("_New profile"));
        } else {
            profile_title.set_text(&profile_name);
        }
        expander_row.add_prefix(&profile_title);
        let id_row = self.create_action_row(&profile_id.to_string());
        id_row.set_visible(false);

        let profile_name_row = self.create_entry_row(&gettext("_Profile name"), &profile_name);
        let profile_name_image_info = gtk::Image::from_icon_name("emblem-important-symbolic");
        profile_name_image_info.add_css_class("warning");
        profile_name_image_info.set_visible(false);
        profile_name_image_info.set_tooltip_text(Some(&gettext("_Name already used")));
        profile_name_row.add_suffix(&profile_name_image_info);

        let email_row = self.create_entry_row(&gettext("_Email address"), &email);
        let email_image_info = gtk::Image::from_icon_name("emblem-important-symbolic");
        email_image_info.add_css_class("error");
        email_image_info.set_visible(false);
        email_image_info.set_tooltip_text(Some(&gettext("_Wrong email")));
        email_row.add_suffix(&email_image_info);

        let username_row = self.create_entry_row(&gettext("_Username"), &username);
        let password_row = self.create_password_row(&gettext("_Token or password"), &password);
        let path_row = self.create_folder_selection_row(&gettext("_Private key path"), &path);

        expander_row.add_row(&id_row);
        expander_row.add_row(&profile_name_row);
        expander_row.add_row(&email_row);
        expander_row.add_row(&self.create_action_row(&gettext("_HTTPS information")));
        expander_row.add_row(&username_row);
        expander_row.add_row(&password_row);
        expander_row.add_row(&self.create_action_row(&gettext("_SSH information")));
        expander_row.add_row(&path_row);

        let row = adw::ActionRow::new();

        let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);

        let save_button = gtk::Button::with_label(&gettext("_Save profile"));
        save_button.set_margin_bottom(10);
        save_button.set_margin_top(10);
        save_button.add_css_class("suggested-action");

        let button_revealer = gtk::Revealer::new();
        button_revealer.set_child(Some(&save_button));
        button_revealer.set_reveal_child(false);
        button_revealer.set_transition_type(gtk::RevealerTransitionType::Crossfade);

        let expander_copy_save_button = expander_row.clone();
        let profile_title_copy = profile_title.clone();
        let self_revealer_copy_name = button_revealer.clone();
        let cloned_self_profile_name = self.clone();
        let email_row_profile_clone = email_row.clone();
        let username_row_profile_clone = username_row.clone();
        let password_row_profile_clone = password_row.clone();
        let path_row_profile_clone = path_row.clone();
        let profile_name_image_info_clone = profile_name_image_info.clone();
        profile_name_row.connect_changed(move |profile| {
            if profile.text().trim() == "" {
                profile_name_image_info_clone.set_visible(false);
                profile_title_copy.add_css_class("dim-label");
                profile_title_copy.set_text(&gettext("_New profile"));
            } else {
                profile_title_copy.remove_css_class("dim-label");
                profile_title_copy.set_text(&profile.text());

                cloned_self_profile_name.imp().obj().emit_by_name::<()>(
                    "unique-name",
                    &[
                        &profile_name_image_info_clone,
                        &profile.text().trim(),
                        &profile_id.to_string(),
                    ],
                );
            }
            // Check if the email is in a correct format :
            let is_email_correct_format =
                EmailAddress::is_valid(email_row_profile_clone.text().trim());
            if !is_email_correct_format && email_row_profile_clone.text().trim() != "" {
                self_revealer_copy_name.set_reveal_child(false);
            } else {
                cloned_self_profile_name.imp().obj().emit_by_name::<()>(
                    "profile-modified",
                    &[
                        &profile_id.to_string().trim(),
                        &profile.text().trim(),
                        &email_row_profile_clone.text().trim(),
                        &username_row_profile_clone.text().trim(),
                        &password_row_profile_clone.text().trim(),
                        &path_row_profile_clone.text().trim(),
                        &self_revealer_copy_name,
                    ],
                );
            }
        });

        let self_revealer_copy_email = button_revealer.clone();
        let cloned_self_email = self.clone();
        let profile_name_row_email_clone = profile_name_row.clone();
        let username_row_email_clone = username_row.clone();
        let password_row_email_clone = password_row.clone();
        let path_row_email_clone = path_row.clone();
        let email_image_cloned = email_image_info.clone();
        email_row.connect_changed(move |row| {
            if row.text().trim() == "" {
                email_image_cloned.set_visible(false);
                cloned_self_email.imp().obj().emit_by_name::<()>(
                    "profile-modified",
                    &[
                        &profile_id.to_string().trim(),
                        &profile_name_row_email_clone.text().trim(),
                        &row.text().trim(),
                        &username_row_email_clone.text().trim(),
                        &password_row_email_clone.text().trim(),
                        &path_row_email_clone.text().trim(),
                        &self_revealer_copy_email,
                    ],
                );
            } else {
                // Check if the email is in a correct format :
                let is_email_correct_format = EmailAddress::is_valid(row.text().trim());
                email_image_cloned.set_visible(!is_email_correct_format);
                if !is_email_correct_format && row.text().trim() != "" {
                    self_revealer_copy_email.set_reveal_child(false);
                } else {
                    cloned_self_email.imp().obj().emit_by_name::<()>(
                        "profile-modified",
                        &[
                            &profile_id.to_string().trim(),
                            &profile_name_row_email_clone.text().trim(),
                            &row.text().trim(),
                            &username_row_email_clone.text().trim(),
                            &password_row_email_clone.text().trim(),
                            &path_row_email_clone.text().trim(),
                            &self_revealer_copy_email,
                        ],
                    );
                }
            }
        });

        let self_revealer_copy_username = button_revealer.clone();
        let cloned_self_username = self.clone();
        let profile_name_row_username_clone = profile_name_row.clone();
        let email_row_username_clone = email_row.clone();
        let password_row_username_clone = password_row.clone();
        let path_row_username_clone = path_row.clone();
        username_row.connect_changed(move |row| {
            let is_email_correct_format =
                EmailAddress::is_valid(email_row_username_clone.text().trim());
            if !is_email_correct_format && email_row_username_clone.text().trim() != "" {
                self_revealer_copy_username.set_reveal_child(false);
            } else {
                cloned_self_username.imp().obj().emit_by_name::<()>(
                    "profile-modified",
                    &[
                        &profile_id.to_string().trim(),
                        &profile_name_row_username_clone.text().trim(),
                        &email_row_username_clone.text().trim(),
                        &row.text().trim(),
                        &password_row_username_clone.text().trim(),
                        &path_row_username_clone.text().trim(),
                        &self_revealer_copy_username,
                    ],
                );
            }
        });

        let self_revealer_copy_password = button_revealer.clone();
        let cloned_self_password = self.clone();
        let profile_name_row_password_clone = profile_name_row.clone();
        let email_row_password_clone = email_row.clone();
        let username_row_password_clone = username_row.clone();
        let path_row_password_clone = path_row.clone();
        password_row.connect_changed(move |row| {
            let is_email_correct_format =
                EmailAddress::is_valid(email_row_password_clone.text().trim());
            if !is_email_correct_format && email_row_password_clone.text().trim() != "" {
                self_revealer_copy_password.set_reveal_child(false);
            } else {
                cloned_self_password.imp().obj().emit_by_name::<()>(
                    "profile-modified",
                    &[
                        &profile_id.to_string().trim(),
                        &profile_name_row_password_clone.text().trim(),
                        &email_row_password_clone.text().trim(),
                        &username_row_password_clone.text().trim(),
                        &row.text().trim(),
                        &path_row_password_clone.text().trim(),
                        &self_revealer_copy_password,
                    ],
                );
            }
        });

        let self_revealer_copy_path = button_revealer.clone();
        let cloned_self_path = self.clone();
        let profile_name_row_path_clone = profile_name_row.clone();
        let email_row_path_clone = email_row.clone();
        let username_row_path_clone = username_row.clone();
        let password_row_path_clone = password_row.clone();
        path_row.connect_changed(move |row| {
            let is_email_correct_format =
                EmailAddress::is_valid(email_row_path_clone.text().trim());
            if !is_email_correct_format && email_row_path_clone.text().trim() != "" {
                self_revealer_copy_path.set_reveal_child(false);
            } else {
                cloned_self_path.imp().obj().emit_by_name::<()>(
                    "profile-modified",
                    &[
                        &profile_id.to_string().trim(),
                        &profile_name_row_path_clone.text().trim(),
                        &email_row_path_clone.text().trim(),
                        &username_row_path_clone.text().trim(),
                        &password_row_path_clone.text().trim(),
                        &row.text().trim(),
                        &self_revealer_copy_path,
                    ],
                );
            }
        });

        let self_button_copy = self.clone();
        let self_revealer_copy_button = button_revealer.clone();
        let cloned_profile_title = profile_title.clone();
        let cloned_profile_row = profile_name_row.clone();
        save_button.connect_clicked(move |_button| {
            expander_copy_save_button.set_expanded(false);
            self_button_copy.imp().obj().emit_by_name::<()>(
                "save-profile",
                &[
                    &profile_id.to_string().trim(),
                    &profile_name_row.text().trim(),
                    &email_row.text().trim(),
                    &username_row.text().trim(),
                    &password_row.text().trim(),
                    &path_row.text().trim(),
                    &cloned_profile_title,
                    &cloned_profile_row,
                ],
            );
            self_revealer_copy_button.set_reveal_child(false);
        });

        let delete_button = gtk::Button::with_label(&gettext("_Delete profile"));
        delete_button.set_margin_bottom(10);
        delete_button.set_margin_top(10);
        delete_button.add_css_class("destructive-action");
        let expander_copy_delete_button = expander_row.clone();
        let self_copy = self.clone();
        delete_button.connect_clicked(move |_button| {
            self_copy.imp().obj().emit_by_name::<()>(
                "delete-profile",
                &[&expander_copy_delete_button, &profile_id.to_string()],
            )
        });

        button_box.append(&button_revealer);
        button_box.append(&delete_button);

        row.add_suffix(&button_box);

        expander_row.add_row(&row);

        expander_row.set_expanded(is_expanded);

        self.imp().git_profiles.insert(&expander_row, 0);
    }

    /**
     * Used to create an entry row.
     */
    pub fn create_entry_row(&self, title: &str, text: &str) -> adw::EntryRow {
        let entry_row = adw::EntryRow::new();
        entry_row.set_title(title);
        entry_row.set_text(text);
        return entry_row;
    }

    /**
     * Used to create an action row.
     */
    pub fn create_action_row(&self, title: &str) -> adw::ActionRow {
        let action_row = adw::ActionRow::new();
        action_row.set_title(title);
        action_row.add_css_class("heading");
        return action_row;
    }

    /**
     * Used to create a password row.
     */
    pub fn create_password_row(&self, title: &str, text: &str) -> adw::PasswordEntryRow {
        let password_row = adw::PasswordEntryRow::new();
        password_row.set_title(title);
        password_row.set_text(text);
        return password_row;
    }

    /**
     * Used to create a folder selection row.
     */
    pub fn create_folder_selection_row(&self, title: &str, text: &str) -> adw::EntryRow {
        let folder_row = adw::EntryRow::new();
        let folder_button = gtk::Button::from_icon_name("folder-symbolic");
        folder_button.set_margin_bottom(10);
        folder_button.set_margin_top(10);
        folder_button.add_css_class("flat");

        let cloned_self = self.clone();
        let cloned_folder_row = folder_row.clone();
        folder_button.connect_clicked(move |_button| {
            cloned_self
                .imp()
                .obj()
                .emit_by_name::<()>("select-location", &[&cloned_folder_row]);
        });

        folder_row.add_suffix(&folder_button);
        folder_row.set_title(title);
        folder_row.set_text(text);

        return folder_row;
    }

    /**
     * Used to delete profile in the UI.
     */
    pub fn delete_git_profile(&self, expander_row: &adw::ExpanderRow) {
        self.imp().git_profiles.remove(expander_row);

        if self.imp().git_profiles.row_at_index(0) == None {
            self.imp().git_profiles.set_visible(false);
            self.imp().status_page.set_visible(true);
        }
    }
}
