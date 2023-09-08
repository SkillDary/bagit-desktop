use super::{changed_file::ChangedFile, changed_folder::ChangedFolder};
use std::fmt;

#[derive(Clone)]
pub struct FileTree {
    tree: Vec<ChangedFile>,
    folders: Vec<ChangedFolder>,
}

impl fmt::Debug for FileTree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "File tree")
    }
}

impl Default for FileTree {
    fn default() -> Self {
        return FileTree {
            tree: Vec::new(),
            folders: Vec::new(),
        };
    }
}

impl FileTree {
    pub fn new(tree: Vec<ChangedFile>, folders: Vec<ChangedFolder>) -> FileTree {
        return FileTree { tree, folders };
    }

    /**
     * Used to get a file from a changed files list.
     */
    pub fn get_changed_file_from_list(&self, file_to_find: &ChangedFile) -> Option<ChangedFile> {
        for file in &self.tree {
            if file.is_same_element(file_to_find) {
                return Some(file.clone());
            }
        }
        return None;
    }

    /**
     * Used to get a file from a changed folders list.
     */
    pub fn get_changed_folder_from_list(&self, folder_path: &str) -> Option<ChangedFolder> {
        for folder in &self.folders {
            if folder.path == folder_path {
                return Some(folder.clone());
            }
        }
        return None;
    }

    /**
     * Update changed files list whith new value.
     */
    pub fn change_file_information(&mut self, new_value: &ChangedFile) {
        for i in 0..self.tree.len() {
            if self.tree[i].is_same_element(new_value) {
                self.tree[i] = new_value.clone();
            }
        }
    }

    pub fn set_selection_of_files_in_folder(&mut self, parent: &str, selected: bool) {
        for i in 0..self.tree.len() {
            if self.tree[i].parent == parent {
                self.tree[i] = ChangedFile::new(
                    parent.to_string(),
                    self.tree[i].name.clone(),
                    self.tree[i].status,
                    selected,
                    self.tree[i].is_opened,
                )
            }
        }
    }

    /**
     * Update expanded value of folder.
     */
    pub fn change_expanded_value_of_folder(&mut self, folder_path: &str, new_is_expanded: bool) {
        for i in 0..self.folders.len() {
            if self.folders[i].path == folder_path {
                self.folders[i] = ChangedFolder::new(folder_path.to_string(), new_is_expanded);
            }
        }
    }

    /**
     * Used to know if all files in a folder are selected.
     */
    pub fn are_all_files_in_folder_selected(&self, folder_path: &str) -> bool {
        for file in &self.tree {
            if file.parent == folder_path && !file.is_selected {
                return false;
            }
        }

        return true;
    }

    /**
     * Used to know if all files are selected.
     */
    pub fn are_all_files_selected(&self) -> bool {
        for file in &self.tree {
            if file.is_selected == false {
                return false;
            }
        }

        return true;
    }

    /// Used to get all selected files.
    pub fn get_selected_files(&self) -> Vec<ChangedFile> {
        let mut changed_files: Vec<ChangedFile> = vec![];

        for file in &self.tree {
            if file.is_selected {
                changed_files.push(file.clone());
            }
        }

        return changed_files;
    }

    /**
     * Used to retrieve the number of selected files.
     */
    pub fn get_number_of_selected_files(&self) -> i32 {
        let mut count = 0;

        for file in &self.tree {
            if file.is_selected {
                count += 1;
            }
        }

        return count;
    }

    /// Used to retrieve the number of changed files.
    pub fn get_number_of_changed_files(&self) -> i32 {
        return self.tree.len().try_into().unwrap();
    }
}
