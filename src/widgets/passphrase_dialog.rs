/* passphrase_dialog.rs
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

//use adw::prelude::WidgetExt;
use adw::subclass::prelude::*;

use adw::prelude::EditableExt;
use adw::prelude::MessageDialogExt;
use adw::prelude::StaticType;
use gettextrs::gettext;
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
    #[template(resource = "/com/skilldary/bagit/desktop/ui/widgets/bagit-gpg-passphrase-dialog.ui")]
    pub struct BagitGpgPassphraseDialog {
        #[template_child]
        pub passphrase_row: TemplateChild<adw::PasswordEntryRow>,
    }

    #[template_callbacks]
    impl BagitGpgPassphraseDialog {
        #[template_callback]
        fn response_cb(&self, choice: Option<&str>) {
            match choice {
                Some(choice) => match choice {
                    "ok" => {
                        self.obj().emit_by_name::<()>(
                            "fetch-passphrase",
                            &[&self.passphrase_row.text().trim()],
                        );
                    }
                    _ => {}
                },
                None => {}
            };
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BagitGpgPassphraseDialog {
        const NAME: &'static str = "BagitGpgPassphraseDialog";
        type Type = super::BagitGpgPassphraseDialog;
        type ParentType = adw::MessageDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitGpgPassphraseDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("fetch-passphrase")
                    .param_types([str::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitGpgPassphraseDialog {}
    impl WindowImpl for BagitGpgPassphraseDialog {}
    impl AdwWindowImpl for BagitGpgPassphraseDialog {}
    impl MessageDialogImpl for BagitGpgPassphraseDialog {}
}

glib::wrapper! {
    pub struct BagitGpgPassphraseDialog(ObjectSubclass<imp::BagitGpgPassphraseDialog>)
        @extends gtk::Widget, gtk::Window, adw::MessageDialog,  @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for BagitGpgPassphraseDialog {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl BagitGpgPassphraseDialog {
    pub fn new(signing_key: &str) -> Self {
        let win: BagitGpgPassphraseDialog = Self::default();
        let body_text = format!("{}\n{}", gettext("_Passphrase key"), signing_key);
        win.set_body(&body_text);

        /*
        let eventctl = gtk::EventControllerKey::new();

        eventctl.connect_key_pressed(|eventctl, keyval, keycode, state| {
            println!("Key : {:?}", keyval.name());

            gtk::Inhibit(false)
        });

        win.add_controller(eventctl);
        */
        win
    }
}
