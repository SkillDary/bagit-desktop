use uuid::Uuid;

#[derive(Clone)]
pub struct BagitRepository {
    pub repository_id: Uuid,
    pub name: String,
    pub path: String,
    pub git_profile_id: Option<Uuid>,
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
