/* application.rs
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
use gtk::prelude::*;
use gtk::{gio, glib};

use crate::config::VERSION;
use crate::preferences::BagitPreferences;
use crate::BagitDesktopWindow;

mod imp {
    use super::*;

    #[derive(Debug, Default)]
    pub struct BagitDesktopApplication {}

    #[glib::object_subclass]
    impl ObjectSubclass for BagitDesktopApplication {
        const NAME: &'static str = "BagitDesktopApplication";
        type Type = super::BagitDesktopApplication;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for BagitDesktopApplication {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            obj.setup_gactions();
            obj.set_accels_for_action("app.quit", &["<primary>q"]);
        }
    }

    impl ApplicationImpl for BagitDesktopApplication {
        // We connect to the activate callback to create a window when the application
        // has been launched. Additionally, this callback notifies us when the user
        // tries to launch a "second instance" of the application. When they try
        // to do that, we'll just present any existing window.
        fn activate(&self) {
            let application = self.obj();
            // Get the current window or create one if necessary
            let window = if let Some(window) = application.active_window() {
                window
            } else {
                let window = BagitDesktopWindow::new(&*application);
                window.set_title(Some("Bagit Desktop"));
                window.upcast()
            };

            // Ask the window manager/compositor to present the window
            window.present();
        }
    }

    impl GtkApplicationImpl for BagitDesktopApplication {}
    impl AdwApplicationImpl for BagitDesktopApplication {}
}

glib::wrapper! {
    pub struct BagitDesktopApplication(ObjectSubclass<imp::BagitDesktopApplication>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl BagitDesktopApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        glib::Object::builder()
            .property("application-id", application_id)
            .property("flags", flags)
            .build()
    }

    fn setup_gactions(&self) {
        let quit_action = gio::ActionEntry::builder("quit")
            .activate(move |app: &Self, _, _| app.quit())
            .build();
        let about_action = gio::ActionEntry::builder("about")
            .activate(move |app: &Self, _, _| app.show_about())
            .build();
        let preferences_action = gio::ActionEntry::builder("preferences")
            .activate(move |app: &Self, _, _| app.show_preferences())
            .build();
        self.add_action_entries([quit_action, about_action, preferences_action]);
    }

    fn show_about(&self) {
        let window = self.active_window().unwrap();
        let about = adw::AboutWindow::builder()
            .transient_for(&window)
            .application_name("bagit-desktop")
            .application_icon("logo-bagit")
            .developer_name("SkillDary")
            .version(VERSION)
            .developers(vec!["Tommy DI LUNA", "Noah PENIN"])
            .copyright("© 2023-2024 SkillDary")
            .build();

        about.present();
    }

    fn show_preferences(&self) {
        let window = self.active_window().unwrap();
        let preferences: BagitPreferences = BagitPreferences::new(&*self);
        preferences.set_title(Some("Bagit Desktop"));
        preferences.set_transient_for(Some(&window));
        preferences.set_modal(true);
        preferences.present();
    }
}
