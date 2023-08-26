use std::fmt;

#[derive(Clone)]
pub struct ChangedFolder {
    pub path: String,
    pub is_expanded: bool,
}

impl fmt::Debug for ChangedFolder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.path, self.is_expanded)
    }
}

impl Default for ChangedFolder {
    fn default() -> Self {
        return ChangedFolder {
            path: String::new(),
            is_expanded: true,
        };
    }
}

impl fmt::Display for ChangedFolder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.path, self.is_expanded)
    }
}

impl ChangedFolder {
    /**
     * Used to create a new ChangedFolder.
     */
    pub fn new(path: String, is_expanded: bool) -> ChangedFolder {
        return ChangedFolder { path, is_expanded };
    }

    /**
     * Used to check if a folder is the same as the current one.
     */
    pub fn is_same_element(&self, changed_folder: &ChangedFolder) -> bool {
        return self.path == changed_folder.path;
    }
}
