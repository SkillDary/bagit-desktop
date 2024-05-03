/* main.rs
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

mod application;
mod clone_repository_page;
mod config;
mod create_repository_page;
mod models;
mod preferences;
mod repository_page;
mod utils;
mod widgets;
mod window;

use directories::ProjectDirs;
use std::fs;
use std::io::Error;
use std::path::PathBuf;
use tracing_appender::rolling::RollingFileAppender;
use tracing_subscriber::{filter, prelude::*};
use walkdir::{DirEntry, WalkDir};

use self::application::BagitDesktopApplication;
use self::window::BagitDesktopWindow;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, setlocale, textdomain, LocaleCategory};
use gtk::prelude::*;
use gtk::{gio, glib};

/// Deletes old log files when we've reached the maximum number of three log files.
fn delete_old_logs(folder_path: PathBuf) {
    tracing::info!("Deleting old log files.");

    // Collect all files matching the pattern "debug.log.YYYY-MM-DD-HH"
    let mut debug_logs: Vec<(DirEntry, String)> = vec![];
    for entry in WalkDir::new(folder_path).into_iter().filter_map(|e| e.ok()) {
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with("debug.log.") {
            let parts: std::str::Split<'_, char> = file_name.split('.');

            let date_time_option = parts.last();

            match date_time_option {
                Some(date_time) => debug_logs.push((entry, date_time.to_string())),
                None => {}
            }
        }
    }

    // Sort the debug logs by date in descending order
    debug_logs.sort_by(|(_, date1), (_, date2)| date2.cmp(date1));

    // Keep the three newest files and delete the rest
    for (index, (entry, file_name)) in debug_logs.iter().enumerate() {
        if index >= 3 {
            let file_path = entry.path();
            let remove_file_result = fs::remove_file(file_path);

            match remove_file_result {
                Ok(_) => tracing::debug!("Old log file removed: debug.log.{}", file_name),
                Err(_) => tracing::error!("Could not remove old log file."),
            }
        }
    }
}

/// Creates the debug log appender.
fn create_debug_log_appender(debug_logs_dir: PathBuf) -> Result<RollingFileAppender, Error> {
    let create_dir_all_result = std::fs::create_dir_all(&debug_logs_dir);

    match create_dir_all_result {
        Ok(_) => {
            let appender = tracing_appender::rolling::hourly(debug_logs_dir, "debug.log");

            return Ok(appender);
        }
        Err(error) => return Err(error),
    }
}

/// Sets up tracing and the debug log files.
fn setup_tracing_and_debug_log() {
    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    // Location of the project folder depending on the OS.
    let project_dir_result = ProjectDirs::from("com", "SkillDary", "Bagit Desktop");

    let project_dir: ProjectDirs;

    match project_dir_result {
        Some(result) => {
            project_dir = result;
        }
        None => {
            tracing_subscriber::registry()
                .with(stdout_log.with_filter(filter::LevelFilter::DEBUG))
                .init();

            tracing::warn!("Could not setup debug log file: project directory was not found or could not be created.");

            return;
        }
    }

    let debug_logs_dir: std::path::PathBuf = project_dir.data_dir().join("logs");

    let debug_log_appender_result = create_debug_log_appender(debug_logs_dir.clone());

    match debug_log_appender_result {
        Ok(debug_log_appender) => {
            let debug_log = tracing_subscriber::fmt::layer().with_writer(debug_log_appender);

            tracing_subscriber::registry()
                .with(stdout_log.with_filter(filter::LevelFilter::DEBUG))
                .with(debug_log.with_filter(filter::LevelFilter::DEBUG))
                .init();
        }
        Err(_) => {
            tracing_subscriber::registry()
                .with(stdout_log.with_filter(filter::LevelFilter::DEBUG))
                .init();

            tracing::warn!("Could not setup debug log file.");
        }
    }

    delete_old_logs(debug_logs_dir);
}

fn main() -> glib::ExitCode {
    setup_tracing_and_debug_log();

    tracing::info!("App launching.");

    // Set up gettext translations
    setlocale(LocaleCategory::LcAll, "");

    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    // Load resources
    let resources: gio::Resource =
        gio::Resource::load(PKGDATADIR.to_owned() + "/bagit-desktop.gresource")
            .expect("Could not load resources");
    gio::resources_register(&resources);

    // Create a new GtkApplication. The application manages our main loop,
    // application windows, integration with the window manager/compositor, and
    // desktop features such as file opening and single-instance applications.
    let app: BagitDesktopApplication = BagitDesktopApplication::new(
        "com.skilldary.bagit.desktop",
        &gio::ApplicationFlags::empty(),
    );

    // Run the application. This function will block until the application
    // exits. Upon return, we have our exit code to return to the shell. (This
    // is the code you see when you do `echo $?` after running a command in a
    // terminal.
    app.run()
}
