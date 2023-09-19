/* https_action_dialog.rs
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

use adw::prelude::EditableExt;
use adw::prelude::StaticType;
use gtk::subclass::widget::CompositeTemplateInitializingExt;
use gtk::{
    gio,
    glib::{self},
};
use gtk::{glib::subclass::Signal, prelude::ObjectExt, template_callbacks};
use once_cell::sync::Lazy;

mod imp {

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/widgets/bagit-https-action-dialog.ui")]
    pub struct BagitHttpsActionDialog {
        #[template_child]
        pub username_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub password_row: TemplateChild<adw::PasswordEntryRow>,
    }

    #[template_callbacks]
    impl BagitHttpsActionDialog {
        #[template_callback]
        fn response_cb(&self, choice: Option<&str>) {
            match choice {
                Some(choice) => match choice {
                    "validate" => {
                        self.obj().emit_by_name::<()>(
                            "push-with-https-informations",
                            &[
                                &self.username_row.text().trim(),
                                &self.password_row.text().trim(),
                            ],
                        );
                    }
                    _ => self.obj().emit_by_name::<()>("cancel", &[]),
                },
                None => {}
            };
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BagitHttpsActionDialog {
        const NAME: &'static str = "BagitHttpsActionDialog";
        type Type = super::BagitHttpsActionDialog;
        type ParentType = adw::MessageDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitHttpsActionDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("push-with-https-informations")
                        .param_types([str::static_type(), str::static_type()])
                        .build(),
                    Signal::builder("cancel").build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitHttpsActionDialog {}
    impl WindowImpl for BagitHttpsActionDialog {}
    impl AdwWindowImpl for BagitHttpsActionDialog {}
    impl MessageDialogImpl for BagitHttpsActionDialog {}
}

glib::wrapper! {
    pub struct BagitHttpsActionDialog(ObjectSubclass<imp::BagitHttpsActionDialog>)
        @extends gtk::Widget, gtk::Window, adw::MessageDialog,  @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for BagitHttpsActionDialog {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl BagitHttpsActionDialog {
    pub fn new(username: &str, password: &str) -> Self {
        let win: BagitHttpsActionDialog = Self::default();
        win.imp().username_row.set_text(&username);
        win.imp().password_row.set_text(&password);
        win
    }
}
