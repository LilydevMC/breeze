#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Failed to get environment variable {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("RconError: {0}")]
    Rcon(#[from] rcon::Error),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Failed to deserialize toml: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
}
