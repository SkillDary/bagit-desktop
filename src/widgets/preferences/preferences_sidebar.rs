/* preferences_sidebar.rs
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
use gtk::glib::subclass::Signal;
use gtk::{glib, prelude::*, CompositeTemplate};
use once_cell::sync::Lazy;

mod imp {

    use gtk::template_callbacks;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(
        resource = "/com/skilldary/bagit/desktop/ui/widgets/preferences/bagit-preferences-sidebar.ui"
    )]
    pub struct BagitPreferencesSideBar {
        #[template_child]
        pub menu: TemplateChild<gtk::ListBox>,
    }

    #[template_callbacks]
    impl BagitPreferencesSideBar {
        #[template_callback]
        fn row_clicked(&self, row: Option<adw::ActionRow>) {
            if row != None {
                let selected_row = row.unwrap();
                self.obj()
                    .emit_by_name::<()>("row-selected", &[&selected_row.index()]);
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitPreferencesSideBar {
        const NAME: &'static str = "BagitPreferencesSideBar";
        type Type = super::BagitPreferencesSideBar;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitPreferencesSideBar {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("row-selected")
                    .param_types([i32::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitPreferencesSideBar {}
    impl BoxImpl for BagitPreferencesSideBar {}
}

glib::wrapper! {
    pub struct BagitPreferencesSideBar(ObjectSubclass<imp::BagitPreferencesSideBar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}
