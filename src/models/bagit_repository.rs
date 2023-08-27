use std::fmt;

use uuid::Uuid;

#[derive(Clone)]
pub struct BagitRepository {
    pub repository_id: Uuid,
    pub name: String,
    pub path: String,
    pub git_profile_id: Option<Uuid>,
}

impl fmt::Display for BagitRepository {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "repo_id: {}\nname: {}\npath: {}\nprofile_id: {}",
            self.repository_id,
            self.name,
            self.path,
            match self.git_profile_id {
                Some(id) => id.to_string(),
                None => "None".to_string(),
            }
        )
    }
}

impl BagitRepository {
    pub fn new(
        repository_id: Uuid,
        name: String,
        path: String,
        git_profile_id: Option<Uuid>,
    ) -> BagitRepository {
        return BagitRepository {
            repository_id: repository_id,
            name: name,
            path: path,
            git_profile_id,
        };
    }
}
