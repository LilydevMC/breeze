use models::config::Config;
use poise::{Framework, FrameworkOptions, PrefixFrameworkOptions, serenity_prelude as serenity};
use serenity::{ClientBuilder, GatewayIntents};
use sqlx::{MySql, Pool};
use tracing::info;

mod commands;
mod database;
mod error;
mod events;
mod models;
mod utils;

struct Data {
    config: Config,
    db: Pool<MySql>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[dotenvy::load(required = false)]
#[tokio::main]
async fn main() -> Result<(), error::ApplicationError> {
    tracing_subscriber::fmt::init();

    let discord_token = std::env::var("DISCORD_TOKEN")?;

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGES;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                commands::servers::server(),
                commands::servers::whitelist::whitelist(),
            ],
            on_error: |error| Box::pin(error::error_handler(error)),
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("wl;".to_string()),
                ..Default::default()
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(events::event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    config: Config::load()?,
                    db: database::create_pool().await?,
                })
            })
        })
        .build();

    let client = ClientBuilder::new(discord_token, intents)
        .framework(framework)
        .await;

    info!("Started breeze!");

    client.unwrap().start().await.unwrap();

    Ok(())
}
