/* branch_management_view.rs
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

use adw::prelude::ActionRowExt;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use git2::{BranchType, Repository};
use gtk::glib;
use gtk::glib::{clone, MainContext, Priority};
use itertools::Itertools;

use gtk::prelude::*;

use crate::utils::repository_utils::RepositoryUtils;

mod imp {

    use std::cell::{Cell, RefCell};

    use super::*;

    use gtk::glib::subclass::Signal;
    use gtk::glib::ObjectExt;
    use gtk::subclass::widget::CompositeTemplateInitializingExt;
    use gtk::template_callbacks;
    use gtk::{glib, CompositeTemplate};
    use once_cell::sync::Lazy;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(
        resource = "/com/skilldary/bagit/desktop/ui/widgets/repository/bagit-branch-management-view.ui"
    )]
    pub struct BagitBranchManagementView {
        #[template_child]
        pub create_branch_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub new_branch_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub branches_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub search_bar: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub all_branches: TemplateChild<gtk::ListBox>,

        pub branches: RefCell<Vec<(String, BranchType, bool)>>,

        pub filtered_branches: RefCell<Vec<(String, BranchType, bool)>>,

        pub is_doing_operations: Cell<bool>,
    }

    #[template_callbacks]
    impl BagitBranchManagementView {
        #[template_callback]
        fn new_branch_name_changed(&self, new_branch_entry_row: &adw::EntryRow) {
            self.create_branch_button
                .set_sensitive(!new_branch_entry_row.text().trim().is_empty());
        }

        #[template_callback]
        fn create_branch(&self, _create_branch_button: &gtk::Button) {
            let new_branch_name = self.new_branch_row.text();
            self.obj()
                .emit_by_name::<()>("create-branch", &[&new_branch_name.trim()]);
        }

        #[template_callback]
        fn search_changed(&self, search_bar: gtk::SearchEntry) {
            self.obj().clear_branches_list();
            let search_entry = search_bar.text();
            self.obj().build_filtered_branch_list(&search_entry);
            let filtered_branches = self.obj().get_filtered_branches();

            if filtered_branches.is_empty() {
                self.branches_stack
                    .set_visible_child_name("no branches page");
            } else {
                self.obj().add_all_branches(filtered_branches);
                self.branches_stack.set_visible_child_name("branches page");
            }
        }

        #[template_callback]
        fn row_clicked(&self, row: Option<adw::ActionRow>) {
            if row != None {
                let selected_row = row.unwrap();
                let index = selected_row.index();
                let branches = self.obj().get_filtered_branches();

                let selected_branch = branches.into_iter().nth(index.try_into().unwrap());

                if let Some(branch) = selected_branch {
                    // If we try to checkout to the same branch (current branch, we do nothing)
                    if branch.2 {
                        // We unselect the choice
                        self.all_branches.unselect_all();
                        return;
                    }

                    let is_remote = branch.1 == BranchType::Remote;
                    self.obj()
                        .emit_by_name::<()>("change-branch", &[&branch.0, &is_remote]);
                }
            }
        }
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitBranchManagementView {
        const NAME: &'static str = "BagitBranchManagementView";
        type Type = super::BagitBranchManagementView;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for BagitBranchManagementView {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
                vec![
                    Signal::builder("change-branch")
                        .param_types([str::static_type(), bool::static_type()])
                        .build(),
                    Signal::builder("create-branch")
                        .param_types([str::static_type()])
                        .build(),
                    Signal::builder("delete-branch")
                        .param_types([str::static_type(), bool::static_type()])
                        .build(),
                ]
            });
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitBranchManagementView {}
    impl BoxImpl for BagitBranchManagementView {}
}
glib::wrapper! {
    pub struct BagitBranchManagementView(ObjectSubclass<imp::BagitBranchManagementView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitBranchManagementView {
    pub fn init_branch_view(&self) {
        self.imp().search_bar.set_text("");
        self.imp().new_branch_row.set_text("");
        self.imp()
            .branches_stack
            .set_visible_child_name("no branches page");
    }

    /// Fetch all branches.
    pub fn fetch_all_branches(&self, repository_path: String) {
        if self.imp().is_doing_operations.get() {
            return;
        }
        self.imp().is_doing_operations.set(true);

        let (sender, receiver) =
            MainContext::channel::<Vec<(String, BranchType, bool)>>(Priority::default());

        thread::spawn(move || {
            let sender = sender.clone();

            let branches = match Repository::open(repository_path) {
                Ok(repo) => match RepositoryUtils::get_all_branches(&repo) {
                    Ok(branches) => branches,
                    Err(_) => vec![],
                },
                Err(_) => vec![],
            };

            sender.send(branches).expect("Cannot send branches");
        });

        receiver.attach(
            None,
            clone!(@weak self as win => @default-return Continue(false),
            move |mut branches| {
                branches.sort_by_key(|branch| !branch.2);
                let current_branches = win.get_branches();

                let need_branches_to_be_updated = branches != current_branches;

                if need_branches_to_be_updated {
                    win.imp()
                    .branches_stack
                    .set_visible_child_name("loading page");
                    win.clear_branches_list();

                    win.imp().branches.replace(branches);

                    let search_entry = win.imp().search_bar.text();
                    win.build_filtered_branch_list(&search_entry);
                    let filtered_branches = win.get_filtered_branches();

                    if filtered_branches.is_empty() {
                        win.imp().branches_stack.set_visible_child_name("no branches page");
                    } else {
                        win.add_all_branches(filtered_branches);
                        win.imp().branches_stack.set_visible_child_name("branches page");
                    }
                }

                win.imp().is_doing_operations.set(false);
                Continue(true)
            }),
        );
    }

    /// Add all branches.
    fn add_all_branches(&self, branches: Vec<(String, BranchType, bool)>) {
        self.imp()
            .all_branches
            .set_selection_mode(gtk::SelectionMode::None);
        for branch in &branches {
            let row = self.build_branch_row(&branch.0, branch.1, branch.2);
            self.imp().all_branches.append(&row);
        }
        self.imp()
            .all_branches
            .set_selection_mode(gtk::SelectionMode::Single);
    }

    /// Build a branch type pill to add to a branch row.
    fn build_branch_type_pill(&self, branch_type: BranchType) -> gtk::Button {
        let pill = gtk::Button::new();
        pill.set_margin_bottom(8);
        pill.set_margin_top(8);
        if branch_type == BranchType::Remote {
            pill.set_label(&gettext("_Remote"));
            pill.add_css_class("warning");
        } else {
            pill.set_label(&gettext("_Local"));
            pill.add_css_class("accent");
        }
        pill.set_can_target(false);
        pill.add_css_class("pill");

        return pill;
    }

    /// Build a delete button with its callback.
    fn build_delete_button(&self, branch_name: &str, branch_type: BranchType) -> gtk::Button {
        let delete_button = gtk::Button::new();
        delete_button.set_icon_name("user-trash-symbolic");
        delete_button.add_css_class("destructive-action");
        delete_button.set_margin_top(8);
        delete_button.set_margin_bottom(8);

        let branch_name_clone = branch_name.to_string();

        delete_button.connect_clicked(clone!(
            @weak self as win
            => move |_| {
                let is_remote = branch_type == BranchType::Remote;
                win.emit_by_name::<()>("delete-branch", &[&branch_name_clone, &is_remote]);
            }
        ));

        return delete_button;
    }

    /// Build a branch row.
    fn build_branch_row(
        &self,
        branch_name: &str,
        branch_type: BranchType,
        is_head: bool,
    ) -> adw::ActionRow {
        let row = adw::ActionRow::builder().title(branch_name).build();
        row.set_title_lines(1);

        if is_head {
            let head_icon = gtk::Image::builder()
                .icon_name("panel-modified-symbolic")
                .tooltip_text(&gettext("_Current branch"))
                .build();

            row.add_prefix(&head_icon);
        }

        let branch_pill = self.build_branch_type_pill(branch_type);
        let delete_button = self.build_delete_button(branch_name, branch_type);

        let delete_button_revealer = gtk::Revealer::new();
        delete_button_revealer.set_child(Some(&delete_button));
        delete_button_revealer.set_transition_type(gtk::RevealerTransitionType::SlideRight);

        let end_box = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        end_box.append(&branch_pill);
        end_box.append(&delete_button_revealer);

        let controller = gtk::EventControllerMotion::new();
        controller.connect_enter(clone!(
            @weak delete_button_revealer
            => move |_, _, _| {
                delete_button_revealer.set_reveal_child(true);
        }));
        controller.connect_leave(clone!(
            @weak delete_button_revealer
             => move |_| {
                delete_button_revealer.set_reveal_child(false);
        }));

        row.add_controller(controller);
        row.add_suffix(&end_box);

        return row;
    }

    /// Clear branches list.
    fn clear_branches_list(&self) {
        let mut branch_row = self.imp().all_branches.row_at_index(0);
        while branch_row != None {
            self.imp().all_branches.remove(&branch_row.unwrap());
            branch_row = self.imp().all_branches.row_at_index(0);
        }
    }

    /// Retrieves the filtered branches.
    fn get_filtered_branches(&self) -> Vec<(String, BranchType, bool)> {
        let filtered_branches: Vec<(String, BranchType, bool)> =
            self.imp().filtered_branches.take();
        self.imp()
            .filtered_branches
            .replace(filtered_branches.clone());
        return filtered_branches;
    }

    /// Retrieves branches.
    fn get_branches(&self) -> Vec<(String, BranchType, bool)> {
        let branches = self.imp().branches.take();
        self.imp().branches.replace(branches.clone());
        return branches;
    }

    /// Builds the filtered branch list with a search_entry
    fn build_filtered_branch_list(&self, search_entry: &str) {
        let mut branches = self.get_branches();
        branches.sort_by_key(|branch| !branch.2);

        // If nothing is searched, we set the filtered branch list as the same as the original list:
        if search_entry.trim().is_empty() {
            self.imp().filtered_branches.replace(branches);
            return;
        }

        let filtered_branches = branches
            .into_iter()
            .filter(|branch| self.does_branch_has_researched_info(branch, &search_entry))
            .collect_vec();

        self.imp().filtered_branches.replace(filtered_branches);
    }

    /// Defines if a branch has the researched information we need.
    fn does_branch_has_researched_info(
        &self,
        branch_information: &(String, BranchType, bool),
        search_entry: &str,
    ) -> bool {
        return branch_information.0.contains(search_entry);
    }
}
