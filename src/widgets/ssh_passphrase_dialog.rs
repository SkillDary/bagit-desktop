/* ssh_passphrase_dialog.rs
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

    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/widgets/bagit-ssh-passphrase-dialog.ui")]
    pub struct BagitSshPassphraseDialog {
        #[template_child]
        pub passphrase_row: TemplateChild<adw::PasswordEntryRow>,

        pub username: RefCell<String>,
        pub private_key_path: RefCell<String>,
    }

    #[template_callbacks]
    impl BagitSshPassphraseDialog {
        #[template_callback]
        fn response_cb(&self, choice: Option<&str>) {
            match choice {
                Some(choice) => match choice {
                    "validate" => {
                        self.obj().emit_by_name::<()>(
                            "push-with-passphrase",
                            &[
                                &self.username.take(),
                                &self.private_key_path.take(),
                                &self.passphrase_row.text().trim(),
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
    impl ObjectSubclass for BagitSshPassphraseDialog {
        const NAME: &'static str = "BagitSshPassphraseDialog";
        type Type = super::BagitSshPassphraseDialog;
        type ParentType = adw::MessageDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitSshPassphraseDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("push-with-passphrase")
                        .param_types([str::static_type(), str::static_type(), str::static_type()])
                        .build(),
                    Signal::builder("cancel").build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitSshPassphraseDialog {}
    impl WindowImpl for BagitSshPassphraseDialog {}
    impl AdwWindowImpl for BagitSshPassphraseDialog {}
    impl MessageDialogImpl for BagitSshPassphraseDialog {}
}

glib::wrapper! {
    pub struct BagitSshPassphraseDialog(ObjectSubclass<imp::BagitSshPassphraseDialog>)
        @extends gtk::Widget, gtk::Window, adw::MessageDialog,  @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for BagitSshPassphraseDialog {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl BagitSshPassphraseDialog {
    pub fn new(username: String, private_key_path: String) -> Self {
        let win: BagitSshPassphraseDialog = Self::default();
        win.imp().username.replace(username);
        win.imp().private_key_path.replace(private_key_path);
        win
    }
}
