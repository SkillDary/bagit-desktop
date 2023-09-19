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

use crate::utils::changed_file::ChangedFile;
use crate::utils::changed_folder::ChangedFolder;
use crate::utils::file_tree::FileTree;
use crate::utils::repository_utils::RepositoryUtils;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use git2::Repository;
use git2::{Status, Statuses};
use gtk::glib::subclass::Signal;
use gtk::glib::{clone, SignalHandlerId};
use gtk::pango::EllipsizeMode;
use gtk::{
    gio, glib, prelude::*, Align, CompositeTemplate, Label, ListItem, NoSelection,
    SignalListItemFactory,
};
use once_cell::sync::Lazy;

extern crate chrono;

use crate::utils::git::{
    get_first_commit_id_of_checked_out_branch, get_repository_checked_out_branch_name,
    load_commit_history,
};

use super::CommitObject;
use std::collections::HashMap;
use std::path::Path;

mod imp {
    use gtk::{gio, template_callbacks};
    use std::cell::Cell;
    use std::cell::RefCell;

    use crate::utils::file_tree::FileTree;

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
        pub total_files: TemplateChild<gtk::Label>,
        #[template_child]
        pub select_by_default_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub menu: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub commit_history_list: TemplateChild<gtk::ListView>,
        #[template_child]
        pub commits_sidebar_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub scrolled_window_commit_history: TemplateChild<gtk::ScrolledWindow>,

        pub scroll_handler_id: RefCell<Option<SignalHandlerId>>,

        pub commit_list: RefCell<Option<gio::ListStore>>,
        pub checked_out_branch_name: RefCell<String>,
        pub first_commit_oid_of_commit_list: RefCell<String>,
        pub last_commit_oid_of_commit_list: RefCell<String>,

        pub changed_files: RefCell<FileTree>,
        pub change_from_file: Cell<bool>,
        pub change_from_user: Cell<bool>,
    }

    #[template_callbacks]
    impl BagitCommitsSideBar {
        #[template_callback]
        fn changed_files_button_clicked(&self, _button: &gtk::Button) {
            self.history_button
                .remove_css_class("commits_siderbar_button_selected");
            self.changed_files_button
                .add_css_class("commits_siderbar_button_selected");
            self.commits_sidebar_stack
                .set_transition_type(gtk::StackTransitionType::SlideRight);
            self.commits_sidebar_stack
                .set_visible_child_name("changes page");

            self.obj().emit_by_name::<()>("update-changed-files", &[]);
        }

        #[template_callback]
        fn history_button_clicked(&self, _button: &gtk::Button) {
            self.changed_files_button
                .remove_css_class("commits_siderbar_button_selected");
            self.history_button
                .add_css_class("commits_siderbar_button_selected");

            self.commits_sidebar_stack
                .set_transition_type(gtk::StackTransitionType::SlideLeft);

            self.commits_sidebar_stack
                .set_visible_child_name("history page");

            self.obj().emit_by_name::<()>("see-history", &[]);
        }

        #[template_callback]
        fn select_button_changed(&self, _check_button: &gtk::CheckButton) {
            if self.change_from_user.get() {
                // We update the changed files list :
                self.obj().clear_changed_files_list();
                self.obj().emit_by_name::<()>("update-changed-files", &[]);
            } else {
                self.change_from_user.set(true);
            }
        }

        #[template_callback]
        fn row_clicked(&self, row: Option<adw::ActionRow>) {
            if row != None {
                let selected_row: adw::ActionRow = row.unwrap();
                self.obj()
                    .emit_by_name::<()>("row-selected", &[&selected_row.index()]);
            }
        }

        #[template_callback]
        fn show_commit_view(&self, _button: &gtk::Button) {
            self.obj().emit_by_name::<()>("show-commit-view", &[]);
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
                    Signal::builder("update-git-action-button")
                        .param_types([i32::static_type()])
                        .build(),
                    Signal::builder("update-changed-files").build(),
                    Signal::builder("see-history").build(),
                    Signal::builder("show-commit-view").build(),
                    Signal::builder("update-file-information-label")
                        .param_types([i32::static_type()])
                        .build(),
                    Signal::builder("discard-file")
                        .param_types([str::static_type()])
                        .build(),
                    Signal::builder("discard-folder")
                        .param_types([str::static_type()])
                        .build(),
                ]
            });

            SIGNALS.as_ref()
        }
        fn constructed(&self) {
            self.parent_constructed();

            self.change_from_user.set(true);
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

impl BagitCommitsSideBar {
    /// Gets the state of the commit list.
    fn commits(&self) -> gio::ListStore {
        self.imp()
            .commit_list
            .borrow()
            .clone()
            .expect("Could not get current tasks.")
    }

    /// Used to initialize the commits sidebar.
    pub fn init_commits_sidebar(&self) {
        self.select_changed_files_stack();
        self.imp().change_from_file.set(false);
        self.clear_changed_files_list();

        // We force the state of the selection button:
        if !self.imp().select_by_default_button.is_active() {
            self.imp().change_from_user.set(false);
            self.imp().select_by_default_button.set_active(true);
        }
    }

    /// Sets up the commit list by creating a new `gio::ListStore` model to hold commit objects.
    fn setup_commit_list(&self) {
        let model: gio::ListStore = gio::ListStore::new(CommitObject::static_type());

        self.imp().commit_list.replace(Some(model));

        let selection_model: NoSelection = NoSelection::new(Some(self.commits()));
        self.imp()
            .commit_history_list
            .set_model(Some(&selection_model));
    }

    /// Sets up a `SignalListItemFactory` for creating custom commit list item views.
    fn setup_commit_list_factory(&self) {
        let factory: SignalListItemFactory = SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item: &glib::Object| {
            let title: Label = Label::new(Some("Title"));
            let subtitle: Label = Label::new(Some("Subtitle"));

            let row: gtk::Box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            row.set_valign(gtk::Align::Center);
            row.set_hexpand(true);

            row.set_margin_top(12);
            row.set_margin_bottom(12);

            let text_box: gtk::Box = gtk::Box::new(gtk::Orientation::Vertical, 8);
            text_box.set_hexpand(true);

            title.add_css_class("heading");
            title.set_property("halign", Align::Start);
            title.set_ellipsize(EllipsizeMode::End);
            subtitle.add_css_class("caption");
            subtitle.set_property("halign", Align::Start);
            subtitle.set_ellipsize(EllipsizeMode::End);

            text_box.append(&title);
            text_box.append(&subtitle);

            row.append(&text_box);

            let local_image: gtk::Image = gtk::Image::from_icon_name("arrow3-up-symbolic");
            local_image.set_pixel_size(24);
            local_image.set_tooltip_text(Some(&gettext("_Commit not pushed")));
            local_image.add_css_class("warning");
            local_image.set_visible(false);
            local_image.set_halign(gtk::Align::End);
            local_image.set_hexpand(true);
            row.append(&local_image);

            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&row));
        });

        factory.connect_bind(move |_, list_item: &glib::Object| {
            // Get `CommitObject` from `ListItem`
            let commit_object: CommitObject = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<CommitObject>()
                .expect("The item has to be a `CommitObject`.");

            // Get `title` from `ListItem`
            let title: Label = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("The child has to be a `Box`.")
                .first_child()
                .and_downcast::<gtk::Box>()
                .expect("First child of `Box` has to be a `Box`.")
                .first_child()
                .and_downcast::<gtk::Label>()
                .expect("First child of `Box` has to be a `Label`.");

            // Get `subtitle` from `ListItem`
            let subtitle: Label = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("The child has to be a `Box`.")
                .first_child()
                .and_downcast::<gtk::Box>()
                .expect("First child of `Box` has to be a `Box`.")
                .last_child()
                .and_downcast::<gtk::Label>()
                .expect("Last child of `Box` has to be a `Label`.");

            // Get 'is_pushed image' from 'ListItem'
            let is_pushed_image: gtk::Image = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<gtk::Box>()
                .expect("The child has to be a `Box`.")
                .last_child()
                .and_downcast::<gtk::Image>()
                .expect("Last child of `Box` has to be an `Image`.");

            title.set_label(&commit_object.title());
            subtitle.set_label(&commit_object.subtitle());
            is_pushed_image.set_visible(!commit_object.is_pushed())
        });

        // TODO: Unbind.

        self.imp().commit_history_list.set_factory(Some(&factory));
    }

    /// Adds commits to the commit history.
    pub fn add_commits_to_history(
        &self,
        nb_commits_to_load: i32,
        selected_repository_path: String,
    ) {
        let repository: Repository = Repository::open(selected_repository_path).unwrap();

        let checked_out_branch_name = get_repository_checked_out_branch_name(&repository);

        let branch: git2::Branch<'_> = repository
            .find_branch(&checked_out_branch_name, git2::BranchType::Local)
            .unwrap();

        let starting_commit_id: String = self.imp().last_commit_oid_of_commit_list.take();

        let newly_loaded_commits: Vec<CommitObject> = load_commit_history(
            &repository,
            branch,
            starting_commit_id.to_string(),
            nb_commits_to_load,
        );

        if self.commits().n_items() == 0 {
            let commits_to_push: i32 = newly_loaded_commits
                .iter()
                .filter(|&commit| !commit.is_pushed())
                .count()
                .try_into()
                .unwrap();

            self.emit_by_name::<()>("update-git-action-button", &[&commits_to_push]);
        }

        // If there is no new commit, we don't go any further.
        if newly_loaded_commits.is_empty() {
            self.imp()
                .last_commit_oid_of_commit_list
                .replace(starting_commit_id);

            return;
        }

        let new_starting_commit_id: String = newly_loaded_commits
            .last()
            .expect("Could not get last commit.")
            .commit_id();

        self.imp()
            .last_commit_oid_of_commit_list
            .replace(new_starting_commit_id);

        self.commits().extend(newly_loaded_commits);
    }

    /// Sets up the callback for the infinite scroll.
    fn setup_infinite_scroll(&self, selected_repository_path: String) {
        let self_clone: BagitCommitsSideBar = self.clone();

        let handler_id: Option<SignalHandlerId> = self.imp().scroll_handler_id.take();

        if handler_id.is_some() {
            self.imp()
                .scrolled_window_commit_history
                .disconnect(handler_id.unwrap());
        }

        let new_handler_id = self
            .imp()
            .scrolled_window_commit_history
            .connect_edge_reached(move |_, pos: gtk::PositionType| match pos {
                gtk::PositionType::Bottom => {
                    self_clone.add_commits_to_history(25, selected_repository_path.clone());
                }
                _ => {}
            });

        self.imp().scroll_handler_id.replace(Some(new_handler_id));
    }

    /// Sets up the first and last commit, used for keeping track of the commit list and to
    /// update it.
    fn setup_first_and_last_commit(&self, selected_repository_path: String) {
        self.imp()
            .last_commit_oid_of_commit_list
            .replace("".to_string());

        let repository: Repository = Repository::open(&selected_repository_path).unwrap();

        let checked_out_branch_name = get_repository_checked_out_branch_name(&repository);

        self.imp()
            .checked_out_branch_name
            .replace(checked_out_branch_name);

        let first_commit_id = get_first_commit_id_of_checked_out_branch(&repository);

        match first_commit_id {
            Some(id) => {
                self.imp()
                    .first_commit_oid_of_commit_list
                    .replace(id.to_string());
            }
            None => {}
        }
    }

    /// Initialize the commit list.
    pub fn init_commit_list(&self, selected_repository_path: String) {
        self.setup_first_and_last_commit(selected_repository_path.clone());

        self.setup_commit_list();

        self.setup_commit_list_factory();

        self.add_commits_to_history(25, selected_repository_path.clone());

        self.setup_infinite_scroll(selected_repository_path);
    }

    /// Checks whether the internal checked out branch matches the actual
    /// checked out branch.
    ///
    /// This is necessary in case the user changes branch elsewhere (e.g. in the shell).
    fn is_checked_out_branch_right(&self, repository: &Repository) -> bool {
        let checked_out_branch = get_repository_checked_out_branch_name(repository);

        let checked_out_branch_name = self.imp().checked_out_branch_name.take();

        self.imp()
            .checked_out_branch_name
            .replace(checked_out_branch_name.clone());

        if checked_out_branch != checked_out_branch_name {
            return false;
        }

        return true;
    }

    /// Checks whether the commit list is up-to-date.
    ///
    /// This is necessary in case the user commits from elsewhere (e.g. in the shell).
    fn is_commit_list_up_to_date(&self, repository: &Repository) -> bool {
        let first_commit_oid_of_commit_list = self.imp().first_commit_oid_of_commit_list.take();

        self.imp()
            .first_commit_oid_of_commit_list
            .replace(first_commit_oid_of_commit_list.clone());

        let first_commit_id = get_first_commit_id_of_checked_out_branch(repository);

        match first_commit_id {
            Some(id) => {
                if id.to_string() != first_commit_oid_of_commit_list {
                    return false;
                }
            }
            None => {}
        }

        return true;
    }

    /// Refreshes the commit list if needed.
    ///
    /// E.g. user changed branch or committed from somewhere else.
    pub fn refresh_commit_list_if_needed(&self, selected_repository_path: String) {
        match Repository::open(&selected_repository_path) {
            Ok(repository) => {
                if !self.is_checked_out_branch_right(&repository) {
                    self.init_commit_list(selected_repository_path.clone());
                    return;
                }

                if !self.is_commit_list_up_to_date(&repository) {
                    self.init_commit_list(selected_repository_path.clone());
                    return;
                }
            }
            Err(_) => return,
        };
    }

    /**
     * Used to clear changed files list for UI.
     */
    pub fn clear_changed_ui_files_list(&self) {
        let mut row = self.imp().menu.row_at_index(0);
        while row != None {
            self.imp().menu.remove(&row.unwrap());
            row = self.imp().menu.row_at_index(0);
        }
    }

    /// Used to clear changed list.
    pub fn clear_changed_files_list(&self) {
        self.imp()
            .changed_files
            .replace(FileTree::new(vec![], vec![]));
    }

    /**
     * Used to generate an add button.
     */
    fn generate_add_button(&self, is_selected: bool) -> gtk::CheckButton {
        let add_button = gtk::CheckButton::new();
        add_button.set_active(is_selected);
        add_button.set_visible(is_selected);

        return add_button;
    }

    /**
     * Used to generate a default discard button.
     */
    fn generate_discard_button(&self) -> gtk::Button {
        let discard_button = gtk::Button::from_icon_name("view-refresh-symbolic");
        discard_button.add_css_class("flat");
        discard_button.add_css_class("circular");
        discard_button.set_visible(false);

        return discard_button;
    }

    /**
     * Used to generate folder with files.
     */
    pub fn generate_folder(&self, folder: ChangedFolder, files: Vec<ChangedFile>) {
        let row = adw::ActionRow::new();

        let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let folder_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let revealer = gtk::Revealer::new();
        revealer.set_reveal_child(folder.is_expanded);
        let file_list = gtk::ListBox::new();
        file_list.add_css_class("background");
        let mut file_add_button_list: Vec<gtk::CheckButton> = Vec::new();

        let folder_label = gtk::Label::new(Some(&folder.path));
        //folder_label.set_max_width_chars(20);
        folder_label.set_ellipsize(gtk::pango::EllipsizeMode::End);

        let dropdown_button = gtk::Button::from_icon_name("go-down-symbolic");
        dropdown_button.add_css_class("flat");
        if folder.is_expanded {
            dropdown_button.set_icon_name("go-down-symbolic");
        } else {
            dropdown_button.set_icon_name("go-next-symbolic");
        }
        dropdown_button.connect_clicked(clone!(
            @weak self as win,
            @weak folder_label,
            @weak revealer,
            => move |button| {
                if revealer.is_child_revealed() {
                    button.set_icon_name("go-next-symbolic");
                } else {
                    button.set_icon_name("go-down-symbolic");
                }
                revealer.set_reveal_child(!revealer.is_child_revealed());
                let mut legacy_list = win.imp().changed_files.take();
                legacy_list.change_expanded_value_of_folder(&folder_label.label(), !revealer.is_child_revealed());
                win.imp().changed_files.replace(legacy_list);
            }
        ));

        let choice_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        choice_box.set_hexpand(true);
        choice_box.set_halign(gtk::Align::End);
        choice_box.set_margin_end(12);

        let add_button: gtk::CheckButton;
        {
            let borrowed_tree = self.imp().changed_files.borrow();
            add_button = self
                .generate_add_button(borrowed_tree.are_all_files_in_folder_selected(&folder.path))
        }

        let discard_folder_path = folder.path.clone();
        let discard_button = self.generate_discard_button();
        discard_button.connect_clicked(clone!(
            @weak self as win => move |_button| {
                win.emit_by_name::<()>("discard-folder", &[&discard_folder_path]);
            }
        ));

        choice_box.append(&discard_button);
        choice_box.append(&add_button);

        let controller = gtk::EventControllerMotion::new();
        controller.connect_enter(clone!(
            @weak discard_button,
            @weak add_button,
            @weak folder_box,
            => move |_, _, _| {
            discard_button.set_visible(true);
            add_button.set_visible(true);
            folder_box.add_css_class("headerbar_bg_color");
        }));
        controller.connect_leave(clone!(
            @weak discard_button,
            @weak add_button,
            @weak folder_box
             => move |_| {
                discard_button.set_visible(false);
                add_button.set_visible(add_button.is_active());
                folder_box.remove_css_class("headerbar_bg_color");
        }));
        folder_box.add_controller(controller);

        folder_box.append(&dropdown_button);
        folder_box.append(&folder_label);
        folder_box.append(&choice_box);
        main_box.append(&folder_box);

        for file in &files {
            let new_file_row = self.generate_changed_file(&file, 30, 6, Some(add_button.clone()));
            file_list.append(&new_file_row.0);
            if file.is_opened {
                file_list.select_row(Some(&new_file_row.0));
            }
            file_add_button_list.push(new_file_row.1);
        }
        revealer.set_child(Some(&file_list));

        let clone_add_button_list = file_add_button_list.clone();
        add_button.connect_toggled(clone!(
            @weak self as win
            => move |button| {
                if !win.imp().change_from_file.get() {
                    let mut legacy_list = win.imp().changed_files.take();

                    legacy_list.set_selection_of_files_in_folder(&folder.path,button.is_active());
                    win.emit_by_name::<()>("update-file-information-label", &[&legacy_list.get_number_of_selected_files()]);

                    let are_all_files_selected = legacy_list.are_all_files_selected();
                    if win.imp().select_by_default_button.is_active() != are_all_files_selected {
                        win.imp().change_from_user.set(false);
                        win.imp().select_by_default_button.set_active(are_all_files_selected);
                    }

                    win.imp().changed_files.replace(legacy_list);

                    for file in &clone_add_button_list {
                        file.set_active(button.is_active());
                        file.set_visible(button.is_active());
                    }
                    win.imp().change_from_file.set(false);
                }
        }));

        main_box.append(&revealer);
        row.set_child(Some(&main_box));
        self.imp().menu.append(&row);
    }

    /**
     * Used to add a new changed file.
     */
    pub fn generate_changed_file(
        &self,
        file: &ChangedFile,
        margin_start: i32,
        margin_end: i32,
        parent_folder_add_button: Option<gtk::CheckButton>,
    ) -> (adw::ActionRow, gtk::CheckButton) {
        let row = adw::ActionRow::new();
        let label = gtk::Label::new(Some(&file.name));
        label.set_halign(gtk::Align::Start);
        label.set_margin_top(8);
        label.set_margin_bottom(8);
        //label.set_max_width_chars(15);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);

        let main_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        main_box.set_margin_end(4);
        main_box.set_hexpand(true);

        let choice_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        choice_box.set_hexpand(true);
        choice_box.set_halign(gtk::Align::End);
        choice_box.set_margin_end(margin_end);

        let add_button = self.generate_add_button(file.is_selected);

        let file_clone = file.clone();
        let parent_clone = parent_folder_add_button.clone();
        add_button.connect_toggled(clone!(
            @weak self as win,
            => move |button| {
                win.imp().change_from_file.set(true);
                let mut legacy_list = win.imp().changed_files.take();

                let mut new_file_info = file_clone.clone();
                new_file_info.is_selected = button.is_active();

                legacy_list.change_file_information(&new_file_info);
                win.emit_by_name::<()>("update-file-information-label", &[&legacy_list.get_number_of_selected_files()]);

                let are_all_files_selected = legacy_list.are_all_files_selected();
                if win.imp().select_by_default_button.is_active() != are_all_files_selected {
                    win.imp().change_from_user.set(false);
                    win.imp().select_by_default_button.set_active(are_all_files_selected);
                }

                let cloned_list = legacy_list.clone();

                win.imp().changed_files.replace(legacy_list);
                if parent_clone.is_some() {
                    let parent_unwrap = parent_clone.clone().unwrap();
                    let new_selection = cloned_list.are_all_files_in_folder_selected(&file_clone.parent);
                    if parent_unwrap.is_active() != new_selection {
                        parent_clone.clone().unwrap().set_active(new_selection);
                        parent_clone.clone().unwrap().set_visible(new_selection);
                    }
                }
                win.imp().change_from_file.set(false);
        }));

        let discard_file_clone = file.clone();
        let discard_button = self.generate_discard_button();
        discard_button.connect_clicked(clone!(
            @weak self as win => move |_button| {
                let file_path = if discard_file_clone.parent.is_empty() {
                    discard_file_clone.name.clone()
                } else {
                    RepositoryUtils::build_path_of_file(&discard_file_clone.parent, &discard_file_clone.name)
                };
                win.emit_by_name::<()>("discard-file", &[&file_path]);
            }
        ));

        choice_box.append(&discard_button);
        choice_box.append(&add_button);

        let controller = gtk::EventControllerMotion::new();
        controller.connect_enter(clone!(
            @weak discard_button,
            @weak add_button,
            @weak row
            => move |_, _, _| {
                discard_button.set_visible(true);
                add_button.set_visible(true);
                row.add_css_class("headerbar_bg_color");
        }));
        controller.connect_leave(clone!(
            @weak discard_button,
            @weak add_button,
            @weak row
             => move |_| {
                discard_button.set_visible(false);
                add_button.set_visible(add_button.is_active());
                row.remove_css_class("headerbar_bg_color");
        }));
        row.add_controller(controller);

        let css_class_name: &str;
        let icon_tooltip_text: String;
        let icon_name: &str;

        match file.status.clone() {
            Status::WT_MODIFIED | Status::INDEX_MODIFIED => {
                css_class_name = "warning";
                icon_name = "panel-modified-symbolic";
                icon_tooltip_text = gettext("_Modified file");
            }
            Status::WT_NEW | Status::INDEX_NEW => {
                css_class_name = "success";
                icon_name = "list-add-symbolic";
                icon_tooltip_text = gettext("_New file");
            }
            Status::WT_DELETED | Status::INDEX_DELETED => {
                css_class_name = "error";
                icon_name = "list-remove-symbolic";
                icon_tooltip_text = gettext("_Deleted file");
            }
            Status::WT_TYPECHANGE | Status::INDEX_TYPECHANGE => {
                css_class_name = "warning";
                icon_name = "panel-modified-symbolic";
                icon_tooltip_text = gettext("_File type file");
            }
            Status::WT_RENAMED | Status::INDEX_RENAMED => {
                css_class_name = "warning";
                icon_name = "panel-modified-symbolic";
                icon_tooltip_text = gettext("_Renamed file");
            }
            _ => {
                css_class_name = "warning";
                icon_name = "panel-modified-symbolic";
                icon_tooltip_text = gettext("_Modified file");
            }
        };

        label.add_css_class(&css_class_name);
        let icon = gtk::Image::from_icon_name(&icon_name);
        icon.set_margin_start(margin_start);
        icon.set_tooltip_text(Some(&icon_tooltip_text));
        icon.add_css_class(&css_class_name);

        main_box.append(&icon);
        main_box.append(&label);
        main_box.append(&choice_box);
        row.set_child(Some(&main_box));

        return (row, add_button);
    }

    /**
     * Used to get parent of file.
     */
    fn get_parent_of_file(&self, path: &str) -> String {
        let parent = Path::new(path).parent();
        return if parent.is_some() {
            parent.unwrap().to_str().unwrap().to_owned()
        } else {
            "".to_owned()
        };
    }

    /**
     * Used to get filename of path.
     */
    fn get_filename_of_path(&self, path: &str) -> String {
        let filename = Path::new(path).file_name();
        return if filename.is_some() {
            filename.unwrap().to_str().unwrap().to_owned()
        } else {
            "".to_owned()
        };
    }

    /// Used to update the changed files indicator.
    fn update_changed_files_indicator(&self, total_files: i32) {
        let text = if total_files == 0 {
            gettext("_No changed file")
        } else if total_files == 1 {
            format!("{} {}", total_files, gettext("_Changed file"))
        } else {
            format!("{} {}", total_files, gettext("_Changed files"))
        };
        self.imp().total_files.set_text(&text);
    }

    /**
     * Used to build a HashMap of parent with files.
     */
    pub fn build_hash_map(&self, statuses: Statuses<'_>) -> HashMap<String, Vec<ChangedFile>> {
        let mut hash_map: HashMap<String, Vec<ChangedFile>> = HashMap::new();
        let borrowed_changed_files = self.imp().changed_files.take();
        let mut new_file_list: Vec<ChangedFile> = Vec::new();
        let mut new_folder_list: Vec<ChangedFolder> = Vec::new();

        for i in 0..statuses.len() {
            let change = statuses.get(i).unwrap();
            let path = change.path().unwrap();
            let parent = self.get_parent_of_file(&path);
            let filename = self.get_filename_of_path(&change.path().unwrap());
            let status = change.status();

            // We only take files and folders that ain't in a gitignore file.
            if !status.is_ignored() {
                let mut current_file = ChangedFile::new(
                    parent.clone(),
                    filename,
                    status,
                    self.imp().select_by_default_button.is_active(),
                    false,
                );
                let mut current_folder = ChangedFolder::new(parent.clone(), true);

                let found_file = borrowed_changed_files.get_changed_file_from_list(&current_file);
                let found_folder = borrowed_changed_files.get_changed_folder_from_list(&parent);

                if found_file.is_some() {
                    let unwraped_file = found_file.unwrap();
                    current_file.is_selected = unwraped_file.is_selected;
                    current_file.is_opened = unwraped_file.is_opened;
                }

                if found_folder.is_some() {
                    current_folder.is_expanded = found_folder.unwrap().is_expanded;
                }
                if !hash_map.contains_key(&parent) {
                    hash_map.insert(parent, vec![current_file.clone()]);
                    new_folder_list.push(current_folder.clone());
                } else {
                    hash_map
                        .get_mut(&parent)
                        .unwrap()
                        .push(current_file.clone());
                }
                new_file_list.push(current_file.clone());
            }
        }
        self.update_changed_files_indicator(new_file_list.len().try_into().unwrap());

        let new_file_tree = FileTree::new(new_file_list, new_folder_list);

        self.emit_by_name::<()>(
            "update-file-information-label",
            &[&new_file_tree.get_number_of_selected_files()],
        );

        self.imp().changed_files.replace(new_file_tree);

        return hash_map;
    }

    // Used to select the changed files stack for initializing the page.
    fn select_changed_files_stack(&self) {
        self.imp()
            .history_button
            .remove_css_class("commits_siderbar_button_selected");
        self.imp()
            .changed_files_button
            .add_css_class("commits_siderbar_button_selected");
        self.imp()
            .commits_sidebar_stack
            .set_transition_type(gtk::StackTransitionType::SlideRight);
        self.imp()
            .commits_sidebar_stack
            .set_visible_child_name("changes page");

        self.imp()
            .commits_sidebar_stack
            .set_visible_child_name("changes page");
    }
}
