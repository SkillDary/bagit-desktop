/* repository_page.rs
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

use crate::utils::selected_repository::SelectedRepository;
use crate::widgets::repository::commits_sidebar::BagitCommitsSideBar;
use adw::subclass::prelude::*;
use gtk::glib::closure_local;
use gtk::{glib, prelude::*, template_callbacks, CompositeTemplate};
use itertools::Itertools;
use std::cell::RefCell;

mod imp {
    use gtk::glib::subclass::Signal;
    use once_cell::sync::Lazy;

    use super::*;

    // Object holding the state
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/repository-page.ui")]
    pub struct BagitRepositoryPage {
        #[template_child]
        pub leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub sidebar: TemplateChild<BagitCommitsSideBar>,

        pub selected_repository: RefCell<SelectedRepository>,
    }

    #[template_callbacks]
    impl BagitRepositoryPage {
        #[template_callback]
        fn go_back(&self, _button: gtk::Button) {
            if self.leaflet.is_folded() {
                self.leaflet.navigate(adw::NavigationDirection::Back);
            }
        }

        #[template_callback]
        fn go_home(&self, _button: gtk::Button) {
            self.obj().emit_by_name::<()>("go-home", &[]);
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitRepositoryPage {
        const NAME: &'static str = "BagitRepositoryPage";
        type Type = super::BagitRepositoryPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitRepositoryPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> =
                Lazy::new(|| vec![Signal::builder("go-home").build()]);
            SIGNALS.as_ref()
        }
        fn constructed(&self) {
            self.parent_constructed();
            self.obj().connect_signals();
        }
    }
    impl WidgetImpl for BagitRepositoryPage {}
    impl BoxImpl for BagitRepositoryPage {}
}

glib::wrapper! {
    pub struct BagitRepositoryPage(ObjectSubclass<imp::BagitRepositoryPage>)
        @extends gtk::Widget,gtk::Window,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitRepositoryPage {
    /**
     * Used for connecting differents signals used by template children.
     */
    pub fn connect_signals(&self) {
        self.imp().sidebar.connect_closure(
            "row-selected",
            false,
            closure_local!(@watch self as _win => move |
                _sidebar: BagitCommitsSideBar,
                index: i32
                | {
                    println!("Commit index: {}", index);
                }
            ),
        );

        self.imp().sidebar.connect_closure(
            "see-history",
            false,
            closure_local!(@watch self as _win => move |_sidebar: BagitCommitsSideBar| {
            }),
        );

        self.imp().sidebar.connect_closure(
            "update-changed-files",
            false,
            closure_local!(@watch self as win => move |
                _sidebar: BagitCommitsSideBar
                | {
                    win.update_changed_files();
                }
            ),
        );
    }

    /// Updates the changed files and commit history.
    pub fn update_commits_sidebar(&self) {
        let selected_repository = self.imp().selected_repository.take();

        let selected_repository_path = selected_repository.user_repository.path.clone();

        self.imp().selected_repository.replace(selected_repository);

        self.imp()
            .sidebar
            .refresh_commit_list_if_needed(selected_repository_path);

        self.update_changed_files();
    }

    /**
     * Used to update changed files list.
     */
    fn update_changed_files(&self) {
        let borrowed_repo = self.imp().selected_repository.borrow();
        if borrowed_repo.git_repository.is_some() {
            let repo = borrowed_repo.git_repository.as_ref().unwrap();
            match repo.statuses(None) {
                Ok(statuses) => {
                    self.imp().sidebar.clear_changed_files_list();
                    let hash_map = self.imp().sidebar.build_hash_map(statuses);

                    for key in hash_map.keys().sorted() {
                        let value = &hash_map[key];
                        if key != "" {
                            let borrowed_changed_folders =
                                self.imp().sidebar.imp().changed_files.borrow();
                            let folder = borrowed_changed_folders
                                .get_changed_folder_from_list(&key)
                                .unwrap();
                            self.imp()
                                .sidebar
                                .generate_folder(folder, (&value).to_vec());
                        }
                    }
                    if hash_map.contains_key("") {
                        for file in &hash_map[""] {
                            let new_row = self.imp().sidebar.generate_changed_file(file, 8, None);
                            self.imp().sidebar.imp().menu.append(&new_row.0);
                        }
                    }
                }
                Err(_) => {}
            };
        }
    }
}
