use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub whitelist: WhitelistConfig,
    pub servers: Vec<Server>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WhitelistConfig {
    pub allow_admin: bool,
    pub allowed_roles: Vec<u64>,
    pub ping_roles: Vec<u64>,
    pub request_channel: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Server {
    pub name: String,
    pub id: String,
    pub container_id: String,
    pub address: String,
    pub rcon_port: u16,
    pub rcon_password: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self, crate::error::ApplicationError> {
        let config = std::fs::read_to_string("config.toml")?;
        let config: Config = toml::from_str(&config)?;
        Ok(config)
    }
}
