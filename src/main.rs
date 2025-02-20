use models::config::Config;
use poise::{serenity_prelude as serenity, Framework, FrameworkOptions, PrefixFrameworkOptions};
use serenity::{ClientBuilder, GatewayIntents};
use sqlx::{MySql, Pool};

mod commands;
mod database;
mod error;
mod events;
mod models;
mod util;

struct Data {
    config: Config,
    db: Pool<MySql>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command, prefix_command)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

#[dotenvy::load]
#[tokio::main]
async fn main() -> Result<(), error::ApplicationError> {
    let discord_token = std::env::var("DISCORD_TOKEN")?;

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![ping(), commands::list_servers(), commands::whitelist()],
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

    client.unwrap().start().await.unwrap();

    Ok(())
}
