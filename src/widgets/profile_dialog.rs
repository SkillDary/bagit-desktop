/* preferences.rs
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

use adw::prelude::EditableExt;
use adw::prelude::ListBoxRowExt;
use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use adw::traits::EntryRowExt;
use adw::traits::PreferencesRowExt;
use gettextrs::gettext;
use gtk::{
    gio,
    glib::{self},
};

use crate::models::bagit_git_profile::BagitGitProfile;

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/widgets/bagit-profile-dialog.ui")]
    pub struct BagitProfileDialog {
        #[template_child]
        pub profile_information: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BagitProfileDialog {
        const NAME: &'static str = "BagitProfileDialog";
        type Type = super::BagitProfileDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitProfileDialog {}
    impl WidgetImpl for BagitProfileDialog {}
    impl WindowImpl for BagitProfileDialog {}
    impl AdwWindowImpl for BagitProfileDialog {}
}

glib::wrapper! {
    pub struct BagitProfileDialog(ObjectSubclass<imp::BagitProfileDialog>)
        @extends gtk::Widget, gtk::Window, adw::Window,  @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for BagitProfileDialog {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl BagitProfileDialog {
    pub fn new(profile: &BagitGitProfile) -> Self {
        let win: BagitProfileDialog = Self::default();
        win.show_profile_information(profile);
        win
    }

    /// Used to show all profile information.
    pub fn show_profile_information(&self, profile: &BagitGitProfile) {
        if !profile.profile_name.is_empty() {
            self.add_profile_information_row(&gettext("_Profile name"), &profile.profile_name);
        }
        if !profile.username.is_empty() {
            self.add_profile_information_row(&gettext("_Username"), &profile.username);
        }
        if !profile.email.is_empty() {
            self.add_profile_information_row(&gettext("_Email address"), &profile.email);
        }
        if !profile.password.is_empty() {
            self.add_profile_confidential_row(
                &gettext("_Token or password dialog"),
                &profile.password,
            );
        }
        if !profile.private_key_path.is_empty() {
            self.add_profile_information_row(
                &gettext("_Private key path dialog"),
                &profile.private_key_path,
            );
        }
        if !profile.signing_key.is_empty() {
            self.add_profile_confidential_row(
                &gettext("_Signing key dialog"),
                &profile.signing_key,
            );
        }
    }

    /// Used to add a profile information row.
    pub fn add_profile_information_row(&self, title: &str, text: &str) {
        let row = adw::ActionRow::new();
        row.set_title(title);
        row.set_subtitle(text);

        self.imp().profile_information.append(&row);
    }

    /// Used to addd a profile confidential row.
    pub fn add_profile_confidential_row(&self, title: &str, text: &str) {
        let confidential_row = adw::PasswordEntryRow::new();
        confidential_row.set_activates_default(false);
        confidential_row.set_selectable(false);
        confidential_row.set_editable(false);
        confidential_row.set_title(title);
        confidential_row.set_text(text);

        self.imp().profile_information.append(&confidential_row);
    }
}
