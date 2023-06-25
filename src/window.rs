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

use adw::{
    subclass::prelude::*,
    traits::{ActionRowExt, PreferencesRowExt},
};
use gtk::prelude::*;
use gtk::{gio, glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/window.ui")]
    pub struct BagitDesktopWindow {
        // Template widgets
        #[template_child]
        pub header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub repositories_window: TemplateChild<adw::Clamp>,
        #[template_child]
        pub status_page: TemplateChild<adw::StatusPage>,
        #[template_child]
        pub new_repo_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub existing_repo_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub clone_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub recent_repositories: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub all_repositories: TemplateChild<gtk::ListBox>,

        pub repositories: Vec<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BagitDesktopWindow {
        const NAME: &'static str = "BagitDesktopWindow";
        type Type = super::BagitDesktopWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitDesktopWindow {}

    #[gtk::template_callbacks]
    impl BagitDesktopWindow {
        #[template_callback]
        fn button_clicked(&self, button: &gtk::Button) {
            println!("Callback!");
            self.status_page.set_visible(false);
            self.repositories_window.set_visible(true);
            let new_row = self.create_list_row("my new repo", "~/path/to/my/super/repo");
            self.all_repositories.append(&new_row);
        }

        pub fn create_list_row(&self, repo_name: &str, repo_path: &str) -> adw::ActionRow {
            let new_row: adw::ActionRow = adw::ActionRow::new();
            let row_image: gtk::Image = gtk::Image::new();
            row_image.set_icon_name(Some("go-next-symbolic"));
            new_row.set_title(repo_name);
            new_row.set_subtitle(repo_path);
            new_row.set_height_request(64);
            new_row.add_suffix(&row_image);

            return new_row;
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
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}
