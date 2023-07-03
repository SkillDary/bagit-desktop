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
use gtk::{glib, prelude::*, CompositeTemplate, template_callbacks};
use gtk::glib::subclass::Signal;
use once_cell::sync::Lazy;
use git2::{RemoteCallbacks,Cred};
use std::fs;
use regex::Regex;
use std::{path::Path};

mod imp {
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
        pub https_auth: TemplateChild<gtk::Box>,
        #[template_child]
        pub https_username: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub https_pwd: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub ssh_auth: TemplateChild<gtk::Box>,
        #[template_child]
        pub ssh_username: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub private_key_path: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub passphrase: TemplateChild<adw::PasswordEntryRow>,
    }

    #[template_callbacks]
    impl BagitCloneRepositoryPage {
        #[template_callback]
        fn go_back(&self, _back_button: &gtk::Button) {
            self.obj().emit_by_name::<()>("go-back", &[]);
        }

        #[template_callback]
        fn url_row_changed(&self, url_text_row: &adw::EntryRow) {
            self.clone_button.set_sensitive(self.obj().can_clone_button_be_sensitive());
            // We must check the type of url :
            if self.obj().is_using_https(&url_text_row.text()) {
                self.https_auth.set_visible(true);
                self.ssh_auth.set_visible(false);
            } else {
                self.https_auth.set_visible(false);
                self.ssh_auth.set_visible(true);

            }
        }

        #[template_callback]
        fn location_row_changed(&self, _url_text_row: &adw::EntryRow) {
            self.clone_button.set_sensitive(self.obj().can_clone_button_be_sensitive());
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
            // We create the new folder which will contain the repo :
            let url_text = self.url_row.text();
            let mut new_folder_name = url_text.as_str().split("/").last().unwrap();

            let replaced_text = &new_folder_name.replace(".git", "");
            new_folder_name = replaced_text.trim();

            let new_folder_path = format!("{}/{}",&self.location_row.text(),new_folder_name);
            
            let new_folder = fs::create_dir(&new_folder_path);

            match new_folder {
                Ok(_) => {
                    let url = &self.url_row.text();
                    let obj = self.obj();
                    let callback = if self.obj().is_using_https(&url) {
                        obj.https_callback()
                    } else {
                        obj.ssh_callback()
                    };

                    let mut fo = git2::FetchOptions::new();
                    fo.remote_callbacks(callback);

                    let mut builder = git2::build::RepoBuilder::new();
                    builder.fetch_options(fo);

                    let repository = builder.clone(
                        &self.url_row.text().trim(), 
                        Path::new(&new_folder_path)
                    );
                    match repository {
                        Ok(_) => self.obj().emit_by_name::<()>("add-repository",&[&new_folder_name, &new_folder_path]),
                        Err(e) => {
                            // We must make sure to delete the created folder !
                            let removed_directory = fs::remove_dir_all(&new_folder_path);
                            match removed_directory {
                                Ok(_) => self.obj().emit_by_name::<()>("error",&[&e.to_string()]),
                                Err(error) => self.obj().emit_by_name::<()>("error",&[&error.to_string()])
                            }
                        }
                    }
                },
                Err(e) => self.obj().emit_by_name::<()>("error",&[&e.to_string()])
            }
        
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
                    Signal::builder("error")
                    .param_types([str::static_type()])
                    .build(),
                    Signal::builder("add-repository")
                    .param_types([str::static_type(), str::static_type()])
                    .build()
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
        return (self.imp().url_row.text().trim()!="") && (self.imp().location_row.text().trim()!="");
    }

    /**
     * Check whether user is using https or ssh to clone a repository.
     */
    pub fn is_using_https(&self, url: &str) -> bool {
        let re = Regex::new(r"https://.*").unwrap();
        return re.is_match(url);
    }

    /**
     * Used to create callback for https clone.
     */
    pub fn https_callback(&self) -> RemoteCallbacks<'_> {
        let mut callback = RemoteCallbacks::new();

        callback.credentials(|_url, username, _allowed_type|{
            let http_username = &self.imp().https_username.text();
            let username_to_use = if http_username.is_empty() {
                username.unwrap()
            } else {
                http_username
            };

            Cred::userpass_plaintext(
                username_to_use, 
                &self.imp().https_pwd.text()
            )
        });

        return callback;
    }

    /**
     * Used to create callback for ssh clone.
     */
    pub fn ssh_callback(&self) -> RemoteCallbacks<'_> {
        let mut callback = RemoteCallbacks::new();

        callback.credentials(|_url, username, _allowed_type|{
            let username_to_use = self.imp().https_username.text();
            let passphrase_to_use = self.imp().passphrase.text();

            Cred::ssh_key(
                if username_to_use.is_empty() {
                    username.unwrap()
                } else {
                    &username_to_use
                }, 
                None, 
                Path::new(&self.imp().private_key_path.text()), 
                if passphrase_to_use.is_empty() {
                    None
                } else {
                    Some(&passphrase_to_use)
                }
            )
        });

        return callback;
    }

    /**
     * Used to clear page information.
     */
    pub fn clear_page(&self) {
        self.imp().url_row.set_text("");
        self.imp().location_row.set_text("");
        self.imp().https_username.set_text("");
        self.imp().https_pwd.set_text("");
        self.imp().ssh_username.set_text("");
        self.imp().private_key_path.set_text("");
        self.imp().passphrase.set_text("");

        self.imp().https_auth.set_visible(true);
        self.imp().ssh_auth.set_visible(false);
    }
}
