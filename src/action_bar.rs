/* action_bar.rs
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
/*
use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate, prelude::*};
use crate::glib::subclass::Signal;
use once_cell::sync::Lazy;

mod imp {
    use super::*;

    // Object holding the state
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource="/com/skilldary/bagit/desktop/ui/widgets/bagit-action-bar.ui")]
    pub struct BagitActionBar{
        #[template_child]
        pub new_repo_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub existing_repo_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub clone_button: TemplateChild<gtk::Button>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitActionBar {
        const NAME: &'static str = "BagitActionBar";
        type Type = super::BagitActionBar;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitActionBar {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("clone-repository")
                    .param_types([i32::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }

        fn constructed(&self) {
            self.clone_button.connect_clicked(move |button| {
                println!("CLICKED");
                button.emit_by_name::<()>("clone-repository", &[]);
                println!("CLICKED");
            });
        }
    }
    impl WidgetImpl for BagitActionBar {}
    impl BoxImpl for BagitActionBar {}
}

glib::wrapper! {
    pub struct BagitActionBar(ObjectSubclass<imp::BagitActionBar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}
*/
