/* git_profile_utils.rs
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

use adw::prelude::ButtonExt;
use adw::prelude::GtkWindowExt;
use adw::prelude::WidgetExt;
use adw::traits::ActionRowExt;
use adw::traits::PreferencesRowExt;

use crate::{
    models::bagit_git_profile::BagitGitProfile, widgets::profile_dialog::BagitProfileDialog,
};

pub struct GitProfileUtils {}

impl GitProfileUtils {
    /// Used to build a profile action row.
    pub fn build_profile_row(profile: &BagitGitProfile) -> adw::ActionRow {
        let action_row = adw::ActionRow::new();

        action_row.set_title(&profile.profile_name);

        let profile_button = GitProfileUtils::build_profile_information_button(profile.clone());

        action_row.add_suffix(&profile_button);

        return action_row;
    }

    /// Used to build a button used to open a dialog to see a profile's information.
    pub fn build_profile_information_button(profile: BagitGitProfile) -> gtk::Button {
        let profile_button = gtk::Button::from_icon_name("view-reveal-symbolic");
        profile_button.set_margin_bottom(10);
        profile_button.set_margin_top(10);
        profile_button.add_css_class("flat");

        profile_button.connect_clicked(move |_button| {
            let profile_dialog = BagitProfileDialog::new(&profile);
            profile_dialog.set_modal(true);
            profile_dialog.present();
        });

        return profile_button;
    }
}
