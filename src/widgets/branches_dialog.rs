/* branches_dialog.rs
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

use std::thread;

use adw::prelude::Continue;
use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use gettextrs::gettext;
use git2::Repository;
use gtk::glib::clone;
use gtk::glib::MainContext;
use gtk::glib::Priority;
use gtk::{gio, glib};

use adw::prelude::StaticType;
use gtk::subclass::widget::CompositeTemplateInitializingExt;

use crate::utils::repository_utils::RepositoryUtils;

mod imp {

    use std::cell::{Cell, RefCell};

    use adw::traits::PreferencesRowExt;
    use gtk::{
        glib::subclass::Signal, prelude::ObjectExt, template_callbacks, traits::GtkWindowExt,
    };
    use once_cell::sync::Lazy;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/widgets/bagit-branches-dialog.ui")]
    pub struct BagitBranchesDialog {
        #[template_child]
        pub dialog_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub local_branches: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub remote_branches: TemplateChild<gtk::ListBox>,

        pub repository_path: RefCell<String>,
        pub id_doing_operations: Cell<bool>,
    }

    #[template_callbacks]
    impl BagitBranchesDialog {
        #[template_callback]
        fn local_branch_selected(&self, row: Option<adw::ActionRow>) {
            if row != None {
                let selected_row: adw::ActionRow = row.unwrap();
                self.obj()
                    .emit_by_name::<()>("select-branch", &[&selected_row.title(), &false]);
            }
        }
        #[template_callback]
        fn remote_branch_selected(&self, row: Option<adw::ActionRow>) {
            if row != None {
                let selected_row: adw::ActionRow = row.unwrap();
                self.obj()
                    .emit_by_name::<()>("select-branch", &[&selected_row.title(), &true]);
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BagitBranchesDialog {
        const NAME: &'static str = "BagitBranchesDialog";
        type Type = super::BagitBranchesDialog;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BagitBranchesDialog {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![Signal::builder("select-branch")
                    .param_types([str::static_type(), bool::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }
        fn constructed(&self) {
            self.parent_constructed();

            self.obj().connect_is_active_notify(clone!(
                @weak self as win
                => move |_| {
                    if !win.id_doing_operations.get() {
                        win.obj().fetch_branches();
                    }
            }));
        }
    }
    impl WidgetImpl for BagitBranchesDialog {}
    impl WindowImpl for BagitBranchesDialog {}
    impl AdwWindowImpl for BagitBranchesDialog {}
}

glib::wrapper! {
    pub struct BagitBranchesDialog(ObjectSubclass<imp::BagitBranchesDialog>)
        @extends gtk::Widget, gtk::Window, adw::Window,  @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for BagitBranchesDialog {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl BagitBranchesDialog {
    pub fn new(repo_path: String) -> Self {
        let win: BagitBranchesDialog = Self::default();
        win.imp().repository_path.replace(repo_path);
        win
    }

    /// Used to fetch branches
    pub fn fetch_branches(&self) {
        self.imp().id_doing_operations.set(true);
        self.imp()
            .dialog_stack
            .set_visible_child_name("loading page");
        self.clear_changed_ui_files_list();
        let (local_sender, local_receiver) =
            MainContext::channel::<Result<Vec<(String, bool)>, String>>(Priority::default());
        let (remote_sender, remote_receiver) =
            MainContext::channel::<Result<Vec<(String, bool)>, String>>(Priority::default());

        let repo_path = self.imp().repository_path.borrow().clone();
        thread::spawn(move || {
            let local_sender = local_sender.clone();
            let remote_sender = remote_sender.clone();

            match Repository::open(repo_path) {
                Ok(repo) => {
                    match RepositoryUtils::get_branches(&repo, git2::BranchType::Local) {
                        Ok(branches) => {
                            local_sender
                                .send(Ok(branches))
                                .expect("Cannot send local branches");
                            //win.add_local_branches(branches);
                        }
                        Err(error) => local_sender
                            .send(Err(error.to_string()))
                            .expect("Cannot send error"),
                    };
                    match RepositoryUtils::get_branches(&repo, git2::BranchType::Remote) {
                        Ok(branches) => {
                            remote_sender
                                .send(Ok(branches))
                                .expect("Cannot send local branches");
                            //win.add_remote_branches(branches);
                        }
                        Err(error) => remote_sender
                            .send(Err(error.to_string()))
                            .expect("Cannot send error"),
                    };
                }
                Err(error) => local_sender
                    .send(Err(error.to_string()))
                    .expect("Cannot send error"),
            };
        });

        local_receiver.attach(
            None,
            clone!(@weak self as win => @default-return Continue(false),
                        move |result| {
                            match result {
                                Ok(branches) => {
                                    win.imp().dialog_stack.set_visible_child_name("branch page");
                                    win.add_local_branches(branches)
                                },
                                Err(_) => {}
                            }
                            Continue(true)
                        }
            ),
        );
        remote_receiver.attach(
            None,
            clone!(@weak self as win => @default-return Continue(false),
                        move |result| {
                            match result {
                                Ok(branches) => {
                                    win.imp().dialog_stack.set_visible_child_name("branch page");
                                    win.add_remote_branches(branches)
                                },
                                Err(_) => {}
                            }
                            win.imp().id_doing_operations.set(false);
                            Continue(true)
                        }
            ),
        );
    }

    /// Used to clear the branches lists.
    pub fn clear_changed_ui_files_list(&self) {
        let mut local_row = self.imp().local_branches.row_at_index(0);
        while local_row != None {
            self.imp().local_branches.remove(&local_row.unwrap());
            local_row = self.imp().local_branches.row_at_index(0);
        }

        let mut remote_row = self.imp().remote_branches.row_at_index(0);
        while remote_row != None {
            self.imp().remote_branches.remove(&remote_row.unwrap());
            remote_row = self.imp().remote_branches.row_at_index(0);
        }
    }

    /// Used to build a branch row.
    fn build_branch_row(&self, branch_name: &str, is_head: bool) -> adw::ActionRow {
        let row = adw::ActionRow::builder().title(branch_name).build();

        if is_head {
            let head_icon = gtk::Image::builder()
                .icon_name("panel-modified-symbolic")
                .tooltip_text(&gettext("_Current branch"))
                .build();

            row.add_prefix(&head_icon);
        }

        return row;
    }

    /// Used to add local branches.
    fn add_local_branches(&self, branches: Vec<(String, bool)>) {
        for branch in branches {
            let row = self.build_branch_row(&branch.0, branch.1);
            self.imp().local_branches.append(&row);
        }
    }

    /// Used to add remote branches.
    fn add_remote_branches(&self, branches: Vec<(String, bool)>) {
        for branch in branches {
            let row = self.build_branch_row(&branch.0, branch.1);
            self.imp().remote_branches.append(&row);
        }
    }
}
