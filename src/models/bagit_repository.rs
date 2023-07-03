use uuid::Uuid;

pub struct BagitRepository {
    pub repository_id: Uuid,
    pub name: String,
    pub path: String,
}

impl BagitRepository {
    pub fn new(repository_id: Uuid, name: String, path: String) -> BagitRepository {
        return BagitRepository {
            repository_id: repository_id,
            name: name,
            path: path,
        };
    }
}
