use crate::{Context, Error, models::config::Server};
use bollard::{Docker, secret::ContainerStateStatusEnum};
use chrono::Utc;
use poise::{
    CreateReply,
    serenity_prelude::{
        ButtonStyle, ChannelId, CreateButton, CreateEmbed, CreateEmbedFooter, CreateMessage,
    },
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerCount {
    online: u32,
    max: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerListEntry {
    pub server: Server,
    pub online: bool,
    pub player_count: Option<PlayerCount>,
}

#[poise::command(slash_command, rename = "list-servers")]
pub async fn list_servers(ctx: Context<'_>) -> Result<(), Error> {
    let mut server_list: Vec<ServerListEntry> = vec![];

    match Docker::connect_with_defaults() {
        Ok(docker) => {
            for server in &ctx.data().config.servers {
                let container = docker.inspect_container(&server.container_id, None).await?;

                let mut player_count: Option<PlayerCount> = None;

                match mc_query::status(&server.address, server.query_port).await {
                    Ok(status) => {
                        println!("{:?}", status);
                        player_count = Some(PlayerCount {
                            online: status.players.online,
                            max: status.players.max,
                        });
                    }
                    Err(e) => {
                        println!("{:?}", e);
                    }
                };

                let online =
                    container.state.unwrap().status.unwrap() == ContainerStateStatusEnum::RUNNING;

                server_list.push(ServerListEntry {
                    server: server.clone(),
                    online,
                    player_count,
                });
            }

            // TODO: This needs better formatting, it looks terrible
            let description = server_list
                .iter()
                .map(|server| {
                    format!(
                        "{} **{}** `{}` - {}{}",
                        if server.online { "🟢" } else { "🔴" },
                        server.server.name,
                        server.server.id,
                        if server.online { "Online" } else { "Offline" },
                        if let Some(player_count) = &server.player_count {
                            format!(" - `{}/{}`", player_count.online, player_count.max)
                        } else {
                            "".to_string()
                        }
                    )
                })
                .collect::<Vec<String>>()
                .join("\n");

            ctx.send(
                CreateReply::default().embed(
                    CreateEmbed::new()
                        .title("ℹ️ Servers")
                        .color(0x04a5e5)
                        .description(description),
                ),
            )
            .await?;
        }
        Err(_) => {
            ctx.send(CreateReply::default().content("Failed to connect to Docker daemon."))
                .await?;
        }
    }

    Ok(())
}

#[poise::command(slash_command, subcommands("request"))]
pub async fn whitelist(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command)]
pub async fn request(
    ctx: Context<'_>,
    #[description = "ID of the server you want whitelisted in (use /list-servers to get a list of all server IDs!)"]
    server_id: String,
    #[description = "Your Minecraft username"] minecraft_username: String,
) -> Result<(), Error> {
    let config = &ctx.data().config;

    if !crate::util::validate_minecraft_username(&minecraft_username).await? {
        ctx.send(
            CreateReply::default()
                .ephemeral(true)
                .content("Invalid Minecraft username! Please make sure you entered it correctly."),
        )
        .await?;
        return Ok(());
    }

    let author = ctx.author();
    let author_id = author.id.to_string();

    let request_id = Uuid::new_v4();
    let request_id_s = request_id.to_string();

    let server = config
        .servers
        .iter()
        .find(|server| server.id == server_id)
        .ok_or("Server not found")?; // TODO: Global custom embed for errors like this

    sqlx::query!(
        "
    	INSERT INTO whitelist_request (id, server_id, discord_id, minecraft_username)
    	VALUES ( ?, ?, ?, ? )
    	",
        request_id_s,
        server_id,
        author_id,
        minecraft_username
    )
    .fetch_all(&ctx.data().db)
    .await?;

    let approve_button = CreateButton::new(format!("wlreq-approve-{}", request_id))
        .label("Approve")
        .style(ButtonStyle::Success);

    let deny_button = CreateButton::new(format!("wlreq-deny-{}", request_id))
        .label("Deny")
        .style(ButtonStyle::Danger);

    let request_embed = CreateEmbed::new()
        .title(":bell: Whitelist Request")
		.color(0xdf8e1d)
        .description(format!(
            "<@{}> has requested to be whitelisted on server _{}_!\n\n**Minecraft Username:** `{}`\n**Server ID:** `{}`\n**Container ID:**: `{}`\n**Request ID:** `{}`",
            author.id, server.name, minecraft_username, server.id, server.container_id, request_id
        ))
		.footer(CreateEmbedFooter::new(format!("Requested at {}", Utc::now())));

    let pings = config
        .whitelist
        .ping_roles
        .iter()
        .map(|role_id| format!("<@&{}>", role_id))
        .collect::<Vec<String>>()
        .join(" ");

    let message = CreateMessage::new()
        .add_embed(request_embed)
        .content(pings)
        .button(approve_button)
        .button(deny_button);

    ChannelId::new(config.whitelist.request_channel)
        .send_message(ctx.http(), message)
        .await?;

    ctx.send(CreateReply::default().ephemeral(true).content(format!(
        "Sent whitelist request for server `{}`!",
        server.id
    )))
    .await?;

    Ok(())
}
