/* repositories.rs
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
use gtk::{glib, prelude::*, CompositeTemplate};
use once_cell::sync::Lazy;

mod imp {

    use adw::traits::ActionRowExt;
    use gtk::{glib::subclass::Signal, prelude::ObjectExt, template_callbacks};

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/widgets/bagit-repositories.ui")]
    pub struct BagitRepositories {
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub recent_repositories_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub recent_repositories: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub all_repositories: TemplateChild<gtk::ListBox>,
    }

    #[template_callbacks]
    impl BagitRepositories {
        #[template_callback]
        fn search_changed(&self, search_bar: gtk::SearchEntry) {
            // We need to be sure that the recent repositories are hide if a search has begun:
            self.recent_repositories_revealer
                .set_reveal_child(search_bar.text().trim().is_empty());

            self.obj()
                .emit_by_name::<()>("search-event", &[&search_bar.text().trim()]);
        }
        #[template_callback]
        fn row_clicked(&self, row: Option<adw::ActionRow>) {
            if row != None {
                let selected_row: adw::ActionRow = row.unwrap();
                self.obj().emit_by_name::<()>(
                    "open-repository",
                    &[&selected_row.subtitle().unwrap().split("~").last().unwrap()],
                );
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitRepositories {
        const NAME: &'static str = "BagitRepositories";
        type Type = super::BagitRepositories;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitRepositories {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("open-repository")
                        .param_types([str::static_type()])
                        .build(),
                    Signal::builder("search-event")
                        .param_types([str::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitRepositories {}
    impl BoxImpl for BagitRepositories {}
}

glib::wrapper! {
    pub struct BagitRepositories(ObjectSubclass<imp::BagitRepositories>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitRepositories {
    /// Used to clear all repositories list.
    pub fn clear_all_repositories_ui_list(&self) {
        let mut row = self.imp().all_repositories.row_at_index(0);
        while row != None {
            self.imp().all_repositories.remove(&row.unwrap());
            row = self.imp().all_repositories.row_at_index(0);
        }
    }

    /// Used to clear recent repositories list.
    pub fn clear_recent_repositories_ui_list(&self) {
        let mut row = self.imp().recent_repositories.row_at_index(0);
        while row != None {
            self.imp().recent_repositories.remove(&row.unwrap());
            row = self.imp().recent_repositories.row_at_index(0);
        }
    }
}
