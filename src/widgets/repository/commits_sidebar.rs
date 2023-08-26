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
use adw::subclass::prelude::*;
use gettextrs::gettext;
use git2::{Status, Statuses};
use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gtk::template_callbacks;
use gtk::{glib, prelude::*, CompositeTemplate};
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;

mod imp {

    use std::cell::Cell;

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
        pub menu: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub commits_sidebar_stack: TemplateChild<gtk::Stack>,

        pub changed_files: RefCell<FileTree>,
        pub change_from_file: Cell<bool>,
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
                    Signal::builder("update-changed-files").build(),
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

impl BagitCommitsSideBar {
    /**
     * Used to clear changed files list.
     */
    pub fn clear_changed_files_list(&self) {
        let mut row = self.imp().menu.row_at_index(0);
        while row != None {
            self.imp().menu.remove(&row.unwrap());
            row = self.imp().menu.row_at_index(0);
        }
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
        folder_box.set_margin_end(2);
        let revealer = gtk::Revealer::new();
        revealer.set_reveal_child(folder.is_expanded);
        let file_list = gtk::ListBox::new();
        file_list.add_css_class("background");
        let mut file_add_button_list: Vec<gtk::CheckButton> = Vec::new();

        let folder_label = gtk::Label::new(Some(&folder.path));
        folder_label.set_max_width_chars(20);
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

        let add_button: gtk::CheckButton;
        {
            let borrowed_tree = self.imp().changed_files.borrow();
            add_button = self
                .generate_add_button(borrowed_tree.are_all_files_in_folder_selected(&folder.path))
        }

        let discard_button = self.generate_discard_button();

        choice_box.append(&discard_button);
        choice_box.append(&add_button);

        let controller = gtk::EventControllerMotion::new();
        controller.connect_enter(clone!(
            @weak discard_button,
            @weak add_button,
            @weak folder_box
            => move |_, _, _| {
            discard_button.set_visible(true);
            add_button.set_visible(true);
        }));
        controller.connect_leave(clone!(
            @weak discard_button,
            @weak add_button
             => move |_| {
                discard_button.set_visible(false);
                add_button.set_visible(add_button.is_active());
        }));
        folder_box.add_controller(controller);

        folder_box.append(&dropdown_button);
        folder_box.append(&folder_label);
        folder_box.append(&choice_box);
        main_box.append(&folder_box);

        for file in &files {
            let new_file_row = self.generate_changed_file(&file, 30, Some(add_button.clone()));
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
        parent_folder_add_button: Option<gtk::CheckButton>,
    ) -> (adw::ActionRow, gtk::CheckButton) {
        let row = adw::ActionRow::new();
        let label = gtk::Label::new(Some(&file.name));
        label.set_halign(gtk::Align::Start);
        label.set_margin_top(8);
        label.set_margin_bottom(8);

        let main_box = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        main_box.set_hexpand(true);

        let choice_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        choice_box.set_hexpand(true);
        choice_box.set_halign(gtk::Align::End);

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

        let discard_button = self.generate_discard_button();

        choice_box.append(&discard_button);
        choice_box.append(&add_button);

        let controller = gtk::EventControllerMotion::new();
        controller.connect_enter(clone!(
            @weak discard_button,
            @weak add_button
            => move |_, _, _| {
            discard_button.set_visible(true);
            add_button.set_visible(true);
        }));
        controller.connect_leave(clone!(
            @weak discard_button,
            @weak add_button
             => move |_| {
                discard_button.set_visible(false);
                add_button.set_visible(add_button.is_active());
        }));
        row.add_controller(controller);

        let css_class_name: &str;
        let icon_tooltip_text: String;
        let icon_name: &str;

        match file.status.clone() {
            Status::WT_MODIFIED => {
                css_class_name = "warning";
                icon_name = "panel-modified-symbolic";
                icon_tooltip_text = gettext("_Modified file");
            }
            Status::WT_NEW => {
                css_class_name = "success";
                icon_name = "list-add-symbolic";
                icon_tooltip_text = gettext("_New file");
            }
            Status::WT_DELETED => {
                css_class_name = "error";
                icon_name = "list-remove-symbolic";
                icon_tooltip_text = gettext("_Deleted file");
            }
            Status::WT_TYPECHANGE => {
                css_class_name = "warning";
                icon_name = "panel-modified-symbolic";
                icon_tooltip_text = gettext("_File type file");
            }
            Status::WT_RENAMED => {
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
        label.set_max_width_chars(15);
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);

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
            if status != Status::IGNORED {
                let mut current_file =
                    ChangedFile::new(parent.clone(), filename, status, false, false);
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

        self.imp()
            .changed_files
            .replace(FileTree::new(new_file_list, new_folder_list));

        return hash_map;
    }
}
