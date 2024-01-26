/* file_view.rs
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

use std::io::Read;
use std::path::Path;

use adw::subclass::prelude::*;
use git2::{Repository, Status};
use gtk::glib::clone;
use gtk::glib::subclass::Signal;
use gtk::prelude::{TextBufferExt, TextViewExt, WidgetExt};
use gtk::subclass::widget::CompositeTemplateInitializingExt;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;
use sourceview5::prelude::BufferExt;
use sourceview5::{Buffer, LanguageManager, StyleScheme, StyleSchemeManager};

use crate::utils::repository_utils::RepositoryUtils;

mod imp {

    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/skilldary/bagit/desktop/ui/widgets/repository/bagit-file-view.ui")]
    pub struct BagitFileView {
        #[template_child]
        pub source_view: TemplateChild<sourceview5::View>,

        pub file_folder: RefCell<String>,
        pub file_name: RefCell<String>,

        pub buffer: RefCell<Buffer>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for BagitFileView {
        const NAME: &'static str = "BagitFileView";
        type Type = super::BagitFileView;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }
    impl ObjectImpl for BagitFileView {
        fn signals() -> &'static [Signal] {
            static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| vec![]);
            SIGNALS.as_ref()
        }
    }
    impl WidgetImpl for BagitFileView {}
    impl BoxImpl for BagitFileView {}
}
glib::wrapper! {
    pub struct BagitFileView(ObjectSubclass<imp::BagitFileView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Actionable,
                    gtk::Buildable, gtk::ConstraintTarget;
}

impl BagitFileView {
    /// Shows the content of a file.
    pub fn show_file(&self, file_name: &str, file_content: &str) {
        let buffer = self.get_buffer(file_name);

        buffer.set_text(&file_content);

        self.imp().buffer.replace(buffer.clone());

        self.set_color_theme_depending_on_system_theme();

        self.imp().source_view.set_buffer(Some(&buffer));
    }

    /// Sets the text view color theme depending on the one of the system.
    pub fn set_color_theme_depending_on_system_theme(&self) {
        self.set_color_theme();

        self.settings()
            .connect_gtk_application_prefer_dark_theme_notify(clone!(
                @weak self as win
                => move |_| {
                    win.set_color_theme();
            }));
    }

    /// Sets the color theme used for the view.
    pub fn set_color_theme(&self) {
        let buffer = self.imp().buffer.take();

        let style_scheme_manager = StyleSchemeManager::new();

        let style: Option<&StyleScheme>;

        let scheme_id;

        match self.settings().is_gtk_application_prefer_dark_theme() {
            true => scheme_id = "Adwaita-dark",
            false => scheme_id = "Adwaita",
        }

        let s: StyleScheme;

        if let Some(style_scheme) = style_scheme_manager.scheme(scheme_id) {
            s = style_scheme.clone();
            style = Some(&s);
        } else {
            buffer.set_style_scheme(None);

            self.imp().buffer.replace(buffer);

            return;
        }

        buffer.set_style_scheme(style);

        self.imp().buffer.replace(buffer);
    }

    /// Retrieves a buffer with a language given from a file name.
    fn get_buffer(&self, file_name: &str) -> Buffer {
        let language_manager = LanguageManager::new();
        let found_language = language_manager.guess_language(Some(file_name), None);

        match found_language {
            Some(language) => Buffer::with_language(&language),
            None => Buffer::new(None),
        }
    }

    /// Tries to show a file.
    /// If the file cannot be shown, an error will be returned.
    pub fn define_how_to_show_file_content(
        &self,
        repository_path: &str,
        repository: &Repository,
        parent_folder: &str,
        file_name: &str,
    ) -> Result<(), String> {
        let relative_path = RepositoryUtils::build_path_of_file(&parent_folder, &file_name);
        let absolute_path = RepositoryUtils::build_path_of_file(repository_path, &relative_path);

        let path = Path::new(&relative_path);

        let is_file_deleted = match repository.status_file(&path) {
            Ok(status) => status == Status::WT_DELETED || status == Status::INDEX_DELETED,
            Err(error) => return Err(error.to_string()),
        };

        if is_file_deleted {
            match RepositoryUtils::get_content_of_deleted_file(&repository, &relative_path) {
                Ok(content) => Ok(self.show_file(&file_name, &content)),
                Err(error) => Err(error.to_string()),
            }
        } else {
            match self.get_content_of_present_file(&absolute_path) {
                Ok(content) => Ok(self.show_file(&file_name, &content)),
                Err(error) => Err(error.to_string()),
            }
        }
    }

    /// Retrieves the content of a file.
    /// The file must be present on the system.
    pub fn get_content_of_present_file(
        &self,
        absolute_file_path: &str,
    ) -> Result<String, std::io::Error> {
        let mut file = std::fs::File::open(absolute_file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        return Ok(contents);
    }

    /// Defines the information of the current viewed file.
    pub fn set_file_information(&self, file_folder: &str, file_name: &str) {
        self.imp().file_folder.replace(file_folder.to_string());
        self.imp().file_name.replace(file_name.to_string());
    }

    /// Retrieves the information of the current shown file.
    /// The first information is the folder, the second the file name.
    pub fn get_current_shown_file_information(&self) -> (String, String) {
        let file_folder = self.imp().file_folder.take();
        self.imp().file_folder.replace(file_folder.clone());

        let file_name = self.imp().file_name.take();
        self.imp().file_name.replace(file_name.clone());

        (file_folder, file_name)
    }
}
