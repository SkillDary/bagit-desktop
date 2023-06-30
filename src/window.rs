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

use std::{env, path::PathBuf};

use crate::glib::clone;
use adw::{
    subclass::prelude::*,
    traits::{ActionRowExt, PreferencesRowExt},
};
use gettextrs::gettext;
use git2::Repository;
use gtk::{
    gio,
    glib::{self, MainContext},
};
use gtk::{glib::closure_local, prelude::*};

use crate::action_bar::BagitActionBar;
use crate::repositories::BagitRepositories;

mod imp {
    use crate::action_bar::BagitActionBar;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/window.ui")]
    pub struct BagitDesktopWindow {
        // Template widgets
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub repositories_window: TemplateChild<BagitRepositories>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub action_bar_content: TemplateChild<BagitActionBar>,

        pub repositories: Vec<i32>,
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

        win.clone_button_signal();

        win.add_existing_repository_button_signal();

        win
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
                                let mut folder_name = "";

                                let os = env::consts::OS;

                                // The path format changes depending on the OS.
                                match os {
                                    "linux" | "macOS" | "freebsd" | "dragonfly" |
                                    "netbsd" | "openbsd" | "solaris" => {
                                        folder_name = folder_path_str.split("/").last().unwrap()
                                    },
                                    "windows" => {
                                        folder_name = folder_path_str.split("\\").last().unwrap()
                                    }
                                    _ => println!("Unsupported OS")
                                };

                                win2.add_list_row(
                                    folder_path_str.split("/").last().unwrap(),
                                    folder_path_str
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

        let new_row: adw::ActionRow = adw::ActionRow::new();
        let row_image: gtk::Image = gtk::Image::new();
        row_image.set_icon_name(Some("go-next-symbolic"));
        new_row.set_title(repo_name);
        new_row.set_subtitle(repo_path);
        new_row.set_height_request(64);
        new_row.add_suffix(&row_image);

        self.imp()
            .repositories_window
            .imp()
            .all_repositories
            .append(&new_row);
    }

    /**
     * Used for listenning to the clone button of the BagitActionBar widget.
     */
    fn clone_button_signal(&self) {
        self.imp().action_bar_content.connect_closure(
            "clone-repository",
            false,
            closure_local!(@watch self as win => move |_action_bar_content: BagitActionBar| {
                win.add_list_row("my new repo","~/path/to/my/super/repo");
            }),
        );
    }
}
