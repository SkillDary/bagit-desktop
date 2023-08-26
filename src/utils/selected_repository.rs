use git2::Repository;
use std::fmt;
use uuid::Uuid;

use crate::models::bagit_repository::BagitRepository;

pub struct SelectedRepository {
    pub user_repository: BagitRepository,
    pub git_repository: Option<Repository>,
}

impl fmt::Debug for SelectedRepository {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Hi")
    }
}

impl Default for SelectedRepository {
    fn default() -> Self {
        return SelectedRepository {
            user_repository: BagitRepository::new(
                Uuid::new_v4(),
                String::new(),
                String::new(),
                None,
            ),
            git_repository: None,
        };
    }
}

impl SelectedRepository {
    /**
     * Create a new SelectedRepository with path and found repository.
     */
    pub fn new_with_repository(repository: &BagitRepository) -> SelectedRepository {
        let git_repo: Option<Repository> = match Repository::open(&repository.path) {
            Ok(repo) => Some(repo),
            Err(_) => None,
        };
        return SelectedRepository {
            user_repository: repository.clone(),
            git_repository: git_repo,
        };
    }

    pub fn new(
        user_repository: BagitRepository,
        git_repository: Option<Repository>,
    ) -> SelectedRepository {
        return SelectedRepository {
            user_repository,
            git_repository,
        };
    }
}
