/* commits_sidebar.rs
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
        resource = "/com/skilldary/bagit/desktop/ui/widgets/repository/bagit-commits-sidebar.ui"
    )]
    pub struct BagitCommitsSideBar {
        #[template_child]
        pub changed_files_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub history_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub menu: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub commits_sidebar_stack: TemplateChild<gtk::Stack>,
    }

    #[template_callbacks]
    impl BagitCommitsSideBar {
        #[template_callback]
        fn changed_files_button_clicked(&self, _button: &gtk::Button) {
            println!("See changed files");

            self.history_button
                .remove_css_class("commits_siderbar_button_selected");
            self.changed_files_button
                .add_css_class("commits_siderbar_button_selected");
            self.obj().emit_by_name::<()>("see-changed-files", &[]);

            self.commits_sidebar_stack
                .set_transition_type(gtk::StackTransitionType::SlideRight);

            self.commits_sidebar_stack
                .set_visible_child_name("changes page");
        }
        #[template_callback]
        fn history_button_clicked(&self, _button: &gtk::Button) {
            println!("See history");

            self.changed_files_button
                .remove_css_class("commits_siderbar_button_selected");
            self.history_button
                .add_css_class("commits_siderbar_button_selected");
            self.obj().emit_by_name::<()>("see-history", &[]);

            self.commits_sidebar_stack
                .set_transition_type(gtk::StackTransitionType::SlideLeft);

            self.commits_sidebar_stack
                .set_visible_child_name("history page");
        }

        #[template_callback]
        fn row_clicked(&self, row: Option<adw::ActionRow>) {
            if row != None {
                let selected_row: adw::ActionRow = row.unwrap();
                self.obj()
                    .emit_by_name::<()>("row-selected", &[&selected_row.index()]);
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitCommitsSideBar {
        const NAME: &'static str = "BagitCommitsSideBar";
        type Type = super::BagitCommitsSideBar;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitCommitsSideBar {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("row-selected")
                        .param_types([i32::static_type()])
                        .build(),
                    Signal::builder("see-changed-files").build(),
                    Signal::builder("see-history").build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitCommitsSideBar {}
    impl BoxImpl for BagitCommitsSideBar {}
}

glib::wrapper! {
    pub struct BagitCommitsSideBar(ObjectSubclass<imp::BagitCommitsSideBar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}
