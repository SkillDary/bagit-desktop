/* ssh_action_dialog.rs
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
    #[template(resource = "/com/skilldary/bagit/desktop/ui/widgets/bagit-ssh-action-dialog.ui")]
    pub struct BagitSshActionDialog {
        #[template_child]
        pub username_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub private_key_path: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub passphrase_row: TemplateChild<adw::PasswordEntryRow>,
    }

    #[template_callbacks]
    impl BagitSshActionDialog {
        #[template_callback]
        fn response_cb(&self, choice: Option<&str>) {
            match choice {
                Some(choice) => match choice {
                    "validate" => {
                        self.obj().emit_by_name::<()>(
                            "push-with-ssh-informations",
                            &[
                                &self.username_row.text().trim(),
                                &self.private_key_path.text().trim(),
                                &self.passphrase_row.text().trim(),
                            ],
                        );
                    }
                    _ => {}
                },
                None => {}
            };
        }
        #[template_callback]
        fn select_location(&self, _select_location_button: &gtk::Button) {
            self.obj().emit_by_name::<()>("select-location", &[]);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BagitSshActionDialog {
        const NAME: &'static str = "BagitSshActionDialog";
        type Type = super::BagitSshActionDialog;
        type ParentType = adw::MessageDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitSshActionDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("push-with-ssh-informations")
                        .param_types([str::static_type(), str::static_type(), str::static_type()])
                        .build(),
                    Signal::builder("select-location").build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitSshActionDialog {}
    impl WindowImpl for BagitSshActionDialog {}
    impl AdwWindowImpl for BagitSshActionDialog {}
    impl MessageDialogImpl for BagitSshActionDialog {}
}

glib::wrapper! {
    pub struct BagitSshActionDialog(ObjectSubclass<imp::BagitSshActionDialog>)
        @extends gtk::Widget, gtk::Window, adw::MessageDialog,  @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for BagitSshActionDialog {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl BagitSshActionDialog {
    pub fn new(username: &str, private_key_path: &str) -> Self {
        let win: BagitSshActionDialog = Self::default();
        win.imp().username_row.set_text(&username);
        win.imp().private_key_path.set_text(&private_key_path);
        win
    }
}
