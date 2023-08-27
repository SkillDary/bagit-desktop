use std::fmt;
use uuid::Uuid;

#[derive(Clone)]
pub struct BagitGitProfile {
    pub profile_id: Uuid,
    pub profile_name: String,
    pub email: String,
    pub username: String,
    pub password: String,
    pub private_key_path: String,
    pub signing_key: String,
}

impl fmt::Debug for BagitGitProfile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BagitGitProfile Debug")
    }
}

impl fmt::Display for BagitGitProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "\nprofile_id: {}\nprofile_name: {}\nusername: {}\nprivate_key_path: {}\nsigning_key: {}", 
                self.profile_id,
            self.profile_name,
            self.username,
            self.private_key_path,
            self.signing_key
        )
    }
}

impl BagitGitProfile {
    pub fn new(
        profile_id: Uuid,
        profile_name: String,
        email: String,
        username: String,
        password: String,
        private_key_path: String,
        signing_key: String,
    ) -> BagitGitProfile {
        return BagitGitProfile {
            profile_id,
            profile_name,
            email,
            username,
            password,
            private_key_path,
            signing_key,
        };
    }
}
