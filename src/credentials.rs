// Module: credentials
// Path: ImageUploader/src/credentials.rs

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Credentials {
    pub organization: String,
    pub username: String,
    pub mission: String,
    pub id: Option<String>,
    pub filepath: Option<String>,
}

impl Clone for Credentials {
    fn clone(&self) -> Self {
        Credentials {
            organization: self.organization.clone(),
            username: self.username.clone(),
            mission: self.mission.clone(),
            id: self.id.clone(),
            filepath: self.filepath.clone(),
        }
    }
}
