use serde::{Deserialize, Serialize};
use serde_default_utils::{default_bool, serde_inline_default};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub whitelist: WhitelistConfig,
    pub servers: Vec<Server>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WhitelistConfig {
    #[serde(default = "default_bool::<true>")]
    pub allow_admin: bool,
    pub allowed_roles: Vec<u64>,
    pub ping_roles: Vec<u64>,
    pub request_channel: u64,
    #[serde(default = "default_bool::<true>")]
    pub send_approval_dm: bool,
    #[serde(default = "default_bool::<false>")]
    pub send_denial_dm: bool,
}

#[serde_inline_default]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Server {
    // maybe name shouldn't be required and just default to the id?
    pub name: String,
    pub id: String,
    pub container_id: String,
    #[serde_inline_default("localhost".to_string())]
    pub address: String,
    pub query_port: u16,
    pub rcon_port: u16,
    #[serde(default = "String::new")]
    pub rcon_password: String,
}

impl Config {
    pub fn load() -> Result<Self, crate::error::ApplicationError> {
        let config_path = std::env::var("CONFIG_PATH").unwrap_or("config.toml".to_string());
        let config_str = std::fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }
}
