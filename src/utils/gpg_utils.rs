/* gpg_utils.rs
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

use std::io::Write;

use gpgme::{PassphraseRequest, PinentryMode};

pub struct GpgUtils {}

impl GpgUtils {
    /// Used to sign a string representation of a commit using a signing key.
    /*
    pub fn sign_commit_string(commit_string: &str, signing_key: &str) -> Result<String, String> {
        let mut ctx = match gpgme::Context::from_protocol(gpgme::Protocol::OpenPgp) {
            Ok(c) => c,
            Err(error) => return Err(error.to_string()),
        };
        ctx.set_armor(true);

        let key = match ctx.get_secret_key(signing_key) {
            Ok(k) => k,
            Err(error) => return Err(error.to_string()),
        };

        ctx.add_signer(&key).unwrap();

        let mut output = Vec::new();

        match ctx.sign_detached(commit_string.clone(), &mut output) {
            Ok(_) => match String::from_utf8(output) {
                Ok(string) => return Ok(string),
                Err(error) => return Err(error.to_string()),
            },
            Err(error) => return Err(error.to_string()),
        };
    }
    */

    /// Used to sign a string representation of a commit using a signing key and its corresponding passphrase.
    pub fn sign_commit_string_with_passphrase(
        commit_string: &str,
        signing_key: &str,
        passphrase: &str,
    ) -> Result<String, String> {
        let mut ctx = match gpgme::Context::from_protocol(gpgme::Protocol::OpenPgp) {
            Ok(c) => c,
            Err(error) => return Err(error.to_string()),
        };

        match ctx.set_pinentry_mode(PinentryMode::Loopback) {
            Ok(_) => {}
            Err(error) => return Err(error.to_string()),
        };

        ctx.with_passphrase_provider(
            |_: PassphraseRequest, out: &mut dyn Write| {
                out.write_all(passphrase.as_bytes())?;
                Ok(())
            },
            |ctx| {
                ctx.set_armor(true);

                let key = match ctx.get_secret_key(signing_key) {
                    Ok(k) => k,
                    Err(error) => return Err(error.to_string()),
                };

                ctx.add_signer(&key).unwrap();

                let mut output = Vec::new();

                match ctx.sign_detached(commit_string.clone(), &mut output) {
                    Ok(_) => match String::from_utf8(output) {
                        Ok(string) => return Ok(string),
                        Err(error) => return Err(error.to_string()),
                    },
                    Err(error) => return Err(error.to_string()),
                };
            },
        )
    }
}
