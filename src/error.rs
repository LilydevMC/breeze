use poise::{CreateReply, FrameworkError, serenity_prelude::CreateEmbed};

use crate::{Data, Error};

#[derive(Debug, thiserror::Error)]
pub enum ApplicationError {
    #[error("Failed to get environment variable {0}")]
    EnvVar(#[from] std::env::VarError),
    #[error("RconError: {0}")]
    Rcon(#[from] mc_query::errors::RconProtocolError),
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("SQL error: {0}")]
    SqlError(#[from] sqlx::Error),
    #[error("Failed to run migrations: {0}")]
    SqlMigrate(#[from] sqlx::migrate::MigrateError),
    #[error("Failed to deserialize toml: {0}")]
    TomlDeserialize(#[from] toml::de::Error),
}

pub async fn error_handler(error: FrameworkError<'_, Data, Error>) {
    match error {
        FrameworkError::Command { error, ctx, .. } => {
            tracing::error!(error);
            let embed = CreateEmbed::default()
                .title("⁉️ Error running command")
                .color(0xd20f39)
                .description(format!(
                    "An error occured while running the command: `{}`\n```{:?}```",
                    ctx.command().name,
                    error
                ));

            let _ = ctx
                .send(CreateReply::default().embed(embed).ephemeral(true))
                .await;
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                tracing::error!("{}", e);
            }
        }
    }
}
