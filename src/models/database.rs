use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct WhitelistRequest {
    pub id: String,
    pub server_id: String,
    pub discord_id: String,
    pub minecraft_username: String,
    pub created_at: Option<DateTime<Utc>>,
}
