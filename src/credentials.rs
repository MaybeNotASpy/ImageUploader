// Module: credentials
// Path: ImageUploader/src/credentials.rs

use serde::Deserialize;
use serde_json::from_str;
use std::error::Error;
use tokio_tungstenite::tungstenite::Message;

#[derive(Deserialize)]
pub struct Credentials {
    pub organization: String,
    pub username: String,
    pub mission: String,
    pub id: Option<String>,
    pub filepath: Option<String>,
}

impl Credentials {
    // Decodes the credentials from the message's json data
    pub fn read_from_message(msg: Message) -> Result<Credentials, Box<dyn Error>> {
        let credentials: Credentials = from_str(&msg.into_text()?)?;
        Ok(credentials)
    }
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
