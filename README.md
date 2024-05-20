# Bagit Desktop

Bagit is a Git client that aims to provide an easy way to use most (if not all) of the power of Git.
It is developed in Rust using GTK4 and Adwaita.

<img title="" src="./src/assets/icons/logo-bagit.svg" alt="alt text" width="197" data-align="center">

# 

## Features

- Git profiles: switch between different identities, and see which one you're using at any time

- Branch management: create, delete and search a branch

- Repository configuration: change the URL or the profile used for a repository whenever you want (more configuration to come)

- SSH and GPG keys management: your keys are linked to a local profile and Bagit will know which one to use, if you have entered the passphrase once, you won't need to type it again (unless you have closed the app in the meantime; this is for security reasons)

## Upcoming features

- See commit information

- Repository creation

- Conflict resolution

- Git submodules support

- And more...

## Known issues and limitations

- If the app is used on a repository that is in a certain state (for example, when rebasing), the app may crash.
- When pulling, if any conflict arises, an error will occur.

## Legal

The name "Bagit" and the logo of Bagit are common law trademarks owned by
SkillDary. All other parties are forbidden from using the name and branding of Bagit, as are derivatives of Bagit. Derivatives include, but are not limited to forks and unofficial builds.

The name "SkillDary" is common law trademark owned by the owner of the website "SkillDary.com". All other parties are forbidden from using the name and branding of SkillDary.

### Licence

Copyright 2023 - 2024 SkillDary

This program is free software: you can redistribute it and/or modify it under the terms of the GNU Affero General Public License as published by the Free Software Foundation, version 3 of the License, only. This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more details. You should have received a copy of the GNU Affero General Public License along with this program. If not, see <http://www.gnu.org/licenses/>.

SPDX-License-Identifier: AGPL-3.0-only
