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

use std::path::PathBuf;

use crate::models::bagit_git_profile::BagitGitProfile;
use crate::utils::db::AppDatabase;
use crate::widgets::preferences::{
    preferences_git_profiles::BagitPreferencesGitProfiles,
    preferences_sidebar::BagitPreferencesSideBar,
};
use adw::subclass::prelude::*;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gtk::glib::{clone, MainContext};
use gtk::prelude::FileExt;
use gtk::template_callbacks;
use gtk::traits::{EditableExt, GtkWindowExt, WidgetExt};
use gtk::{
    gio,
    glib::{self, closure_local},
    prelude::ObjectExt,
};
use uuid::Uuid;

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/preferences.ui")]
    pub struct BagitPreferences {
        #[template_child]
        pub leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub sidebar: TemplateChild<BagitPreferencesSideBar>,
        #[template_child]
        pub identities: TemplateChild<BagitPreferencesGitProfiles>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,

        pub app_database: AppDatabase,
    }

    #[template_callbacks]
    impl BagitPreferences {
        #[template_callback]
        fn go_back(&self, _button: gtk::Button) {
            if self.leaflet.is_folded() {
                self.leaflet.navigate(adw::NavigationDirection::Back);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BagitPreferences {
        const NAME: &'static str = "BagitPreferences";
        type Type = super::BagitPreferences;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitPreferences {}
    impl WidgetImpl for BagitPreferences {}
    impl WindowImpl for BagitPreferences {}
    impl ApplicationWindowImpl for BagitPreferences {}
    impl AdwApplicationWindowImpl for BagitPreferences {}
}

glib::wrapper! {
    pub struct BagitPreferences(ObjectSubclass<imp::BagitPreferences>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,        @implements gio::ActionGroup, gio::ActionMap;
}

impl BagitPreferences {
    pub fn new<P: glib::IsA<gtk::Application>>(application: &P) -> Self {
        let win: BagitPreferences = glib::Object::builder::<BagitPreferences>()
            .property("application", application)
            .build();

        win.connect_signals();
        win.fetch_git_profiles();
        win
    }

    /**
     * Used for connecting differents signals used by template children.
     */
    pub fn connect_signals(&self) {
        self.imp().sidebar.connect_closure(
            "row-selected",
            false,
            closure_local!(@watch self as win => move |
                _sidebar: BagitPreferencesSideBar,
                index: i32
                | {
                    match index {
                        0 => {
                            win.imp().stack.set_visible_child_name("identities");
                            if win.imp().leaflet.is_folded() {
                                win.imp().leaflet.navigate(adw::NavigationDirection::Forward);
                            }

                        }
                        _ => {
                            win.imp().stack.set_visible_child_name("identities");
                            if win.imp().leaflet.is_folded() {
                                win.imp().leaflet.navigate(adw::NavigationDirection::Forward);
                            }
                        }
                    }
                }
            ),
        );

        self.imp().identities.connect_closure(
            "can-add-profile",
            false,
            closure_local!(@watch self as win => move |
                _identities: BagitPreferencesGitProfiles
                | {
                    // We verify if the user has not already a profile in creation :
                    let all_profiles = win.imp().app_database.get_all_git_profiles();
                    if win.imp().identities.imp().git_profiles.row_at_index(all_profiles.len().try_into().unwrap()) == None {
                        if win.imp().identities.imp().status_page.is_visible() {
                            win.imp().identities.imp().status_page.set_visible(false);
                            win.imp().identities.imp().git_profiles.set_visible(true);
                        }

                        win.imp().identities.imp().obj()
                            .add_new_git_profile(Uuid::new_v4(), "","", "", "", "", "", true);
                    } else {
                        let toast = adw::Toast::new(&gettext("_New profile awaiting"));
                        win.imp().toast_overlay.add_toast(toast);
                    }
            }),
        );

        self.imp().identities.connect_closure(
            "save-profile",
            false,
            closure_local!(@watch self as win => move |
                _identities: BagitPreferencesGitProfiles,
                profile_id: &str,
                profile_name: &str,
                email: &str,
                username: &str,
                password: &str,
                private_key_path: &str,
                signing_key: &str,
                profile_title: &gtk::Label,
                profile_row: &adw::EntryRow
                | {
                    // We make sure that the profile name is unique :
                    let same_profile_name_number = win.imp().app_database.get_number_of_git_profiles_with_name(
                        &profile_name,
                        &profile_id
                    );
                    let final_profil_name : String =  if same_profile_name_number != 0 {
                        let new_name = format!("{} ({})", profile_name, same_profile_name_number);
                        profile_title.set_text(&new_name);
                        profile_row.set_text(&new_name);
                        new_name
                    } else {
                        profile_name.to_string()
                    };

                    if win.imp().app_database.does_git_profile_exist(profile_id) {
                        win.imp().app_database.update_git_profile(
                            &BagitGitProfile::new(
                                Uuid::parse_str(profile_id).unwrap(),
                                final_profil_name,
                                email.to_string(),
                                username.to_string(),
                                password.to_string(),
                                private_key_path.to_string(),
                                signing_key.to_string()
                            )
                        )
                    } else {
                        win.imp().app_database.add_git_profile(
                            &BagitGitProfile::new(
                                Uuid::parse_str(profile_id).unwrap(),
                                final_profil_name,
                                email.to_string(),
                                username.to_string(),
                                password.to_string(),
                                private_key_path.to_string(),
                                signing_key.to_string()
                            )
                        )
                    }
                }
            ),
        );

        self.imp().identities.connect_closure(
            "profile-modified",
            false,
            closure_local!(@watch self as win => move |
                _identities: BagitPreferencesGitProfiles,
                profile_id: &str,
                profile_name: &str,
                email: &str,
                username: &str,
                password: &str,
                private_key_path: &str,
                signing_key: &str,
                revealer: &gtk::Revealer
                | {
                    revealer.set_reveal_child(!win.imp().app_database.does_git_profile_exist_from_information(
                        profile_id,
                        profile_name,
                        email,
                        username,
                        password,
                        private_key_path,
                        signing_key
                    ) && profile_name != "" && username != "" && email != "");
                }
            ),
        );

        self.imp().identities.connect_closure(
            "delete-profile",
            false,
            closure_local!(@watch self as win => move |
                _identities: BagitPreferencesGitProfiles,
                expander_row: adw::ExpanderRow,
                profile_id: &str,
                | {
                    let ctx: MainContext = glib::MainContext::default();
                    let profile_id_clone = profile_id.to_string();
                    ctx.spawn_local(clone!(@weak win as win2 => async move {
                        let delete_dialog = adw::MessageDialog::new(Some(&win2), Some(&gettext("_Delete profile")), Some(&gettext("_Delete profile confirmation")));
                        delete_dialog.add_response(&gettext("_Cancel"), &gettext("_Cancel"));
                        delete_dialog.add_response(&gettext("_Delete"), &gettext("_Delete"));
                        delete_dialog.set_close_response(&gettext("_Cancel"));
                        delete_dialog.set_response_appearance(&gettext("_Delete"), adw::ResponseAppearance::Destructive);
                        delete_dialog.present();
                        delete_dialog.connect_response(None, move |_dialog, response| {
                            if response == &gettext("_Delete") {
                                win2.imp().identities.delete_git_profile(&expander_row);
                                win2.imp().app_database.delete_git_profile(&profile_id_clone);
                            }
                        });
                    }));
                }
            ),
        );

        self.imp().identities.connect_closure(
            "select-location",
            false,
            closure_local!(@watch self as win => move |
                _identities: BagitPreferencesGitProfiles,
                path_row: adw::EntryRow
                | {
                let ctx: MainContext = glib::MainContext::default();
                ctx.spawn_local(clone!(@weak win as win2 => async move {
                    let dialog = gtk::FileDialog::builder()
                        .accept_label(gettext("_Add"))
                        .modal(true)
                        .title(gettext("_Select location"))
                        .build();

                    if let Ok(res) = dialog.open_future(Some(&win2)).await {
                        path_row.set_text(
                            res.path().unwrap_or(PathBuf::new()).to_str().unwrap()
                        );
                    }
                }));
            }),
        );

        self.imp().identities.connect_closure(
            "unique-name",
            false,
            closure_local!(@watch self as win => move |
                _identities: BagitPreferencesGitProfiles,
                image: gtk::Image,
                profile_name: &str,
                profile_id: &str
                | {
                    let same_profile_name_number = win.imp().app_database.get_number_of_git_profiles_with_name(
                        &profile_name,
                        &profile_id
                    );
                    image.set_visible(same_profile_name_number != 0);
            }),
        );
    }

    /**
     * Used for fetching all git profiles and update view.
     */
    pub fn fetch_git_profiles(&self) {
        let git_profiles = self.imp().app_database.get_all_git_profiles();

        for profile in git_profiles {
            self.imp().identities.imp().status_page.set_visible(false);
            self.imp().identities.imp().git_profiles.set_visible(true);

            self.imp().identities.imp().obj().add_new_git_profile(
                profile.profile_id,
                &profile.profile_name,
                &profile.email,
                &profile.username,
                &profile.password,
                &profile.private_key_path,
                &profile.signing_key,
                false,
            );
        }
    }
}
