use uuid::Uuid;

pub struct BagitGitProfile {
    pub profile_id: Uuid,
    pub profile_name: String,
    pub email: String,
    pub username: String,
    pub password: String,
    pub private_key_path: String,
}

impl BagitGitProfile {
    pub fn new(
        profile_id: Uuid,
        profile_name: String,
        email: String,
        username: String,
        password: String,
        private_key_path: String,
    ) -> BagitGitProfile {
        return BagitGitProfile {
            profile_id,
            profile_name,
            email,
            username,
            password,
            private_key_path,
        };
    }
}
