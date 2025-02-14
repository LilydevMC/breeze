use poise::{serenity_prelude as serenity, Framework, FrameworkOptions, PrefixFrameworkOptions};
use serenity::{ClientBuilder, GatewayIntents};

struct Data {}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[poise::command(slash_command, prefix_command)]
async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.say("Pong!").await?;
    Ok(())
}

#[dotenvy::load(required = false)]
#[tokio::main]
async fn main() {
    let discord_token = std::env::var("DISCORD_TOKEN").unwrap();

    let intents = GatewayIntents::non_privileged() | GatewayIntents::GUILD_MESSAGES;

    let framework = Framework::builder()
        .options(FrameworkOptions {
            commands: vec![ping()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("wl;".to_string()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build();

    let client = ClientBuilder::new(discord_token, intents)
        .framework(framework)
        .await;

    client.unwrap().start().await.unwrap();
}
