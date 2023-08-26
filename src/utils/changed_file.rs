use git2::Status;
use std::fmt;

#[derive(Clone)]
pub struct ChangedFile {
    pub parent: String,
    pub name: String,
    pub status: Status,
    pub is_selected: bool,
    pub is_opened: bool,
}

impl fmt::Debug for ChangedFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Hi")
    }
}

impl Default for ChangedFile {
    fn default() -> Self {
        return ChangedFile {
            parent: String::new(),
            name: String::new(),
            status: Status::WT_MODIFIED,
            is_selected: false,
            is_opened: false,
        };
    }
}

impl fmt::Display for ChangedFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.name, self.is_selected, self.is_opened)
    }
}

impl ChangedFile {
    /**
     * Used to create a new ChangedFile.
     */
    pub fn new(
        parent: String,
        name: String,
        status: Status,
        is_selected: bool,
        is_opened: bool,
    ) -> ChangedFile {
        return ChangedFile {
            parent,
            name,
            status,
            is_selected,
            is_opened,
        };
    }

    /**
     * Used to check if a file is the same as the current one.
     */
    pub fn is_same_element(&self, changed_file: &ChangedFile) -> bool {
        return self.parent == changed_file.parent
            && self.name == changed_file.name
            && self.status == changed_file.status;
    }
}
