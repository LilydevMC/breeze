use crate::{Context, Error, models::config::Server};
use bollard::{Docker, secret::ContainerStateStatusEnum};
use chrono::Utc;
use poise::{CreateReply, serenity_prelude as serenity};
use serde::{Deserialize, Serialize};
use serenity::{
    AutocompleteChoice, ButtonStyle, ChannelId, CreateButton, CreateEmbed, CreateEmbedFooter,
    CreateMessage,
};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerAdditionalInfo {
    players_online: u32,
    players_max: u32,
    version: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerListEntry {
    pub server: Server,
    pub online: bool,
    pub additional_info: Option<ServerAdditionalInfo>,
}

fn create_server_list_fields(servers: Vec<ServerListEntry>) -> Vec<(String, String, bool)> {
    let mut fields: Vec<(String, String, bool)> = vec![];
    for server in servers {
        let additional_info = server.additional_info;

        let field = match additional_info {
            Some(info) => (
                server.server.name,
                format!(
                    "**ID:** `{}`\n**Status:** {}\n**Players:** `{}/{}`\n**Version:** `{}`",
                    server.server.id,
                    if server.online { "Online" } else { "Offline" },
                    info.players_online,
                    info.players_max,
                    info.version
                ),
                false,
            ),
            None => (
                server.server.name,
                format!(
                    "**ID:** `{}`\n**Status:** {}",
                    server.server.id,
                    if server.online { "Online" } else { "Offline" }
                ),
                false,
            ),
        };
        fields.push(field);
    }

    fields
}

async fn autocomplete_server_ids(
    ctx: Context<'_>,
    partial: &str,
) -> impl Iterator<Item = AutocompleteChoice> {
    ctx.data().config.servers.iter().filter_map(move |s| {
        if s.id.starts_with(partial) {
            Some(AutocompleteChoice::new(
                format!("{} ({})", s.name, s.id),
                s.id.clone(),
            ))
        } else {
            None
        }
    })
}

/// List all servers with their status and additional info if available
#[poise::command(slash_command, rename = "list-servers")]
pub async fn list_servers(ctx: Context<'_>) -> Result<(), Error> {
    let mut server_list: Vec<ServerListEntry> = vec![];

    match Docker::connect_with_defaults() {
        Ok(docker) => {
            for server in &ctx.data().config.servers {
                let container = docker.inspect_container(&server.container_id, None).await?;

                let mut additional_info: Option<ServerAdditionalInfo> = None;

                if let Ok(status) = mc_query::status(&server.address, server.query_port).await {
                    additional_info = Some(ServerAdditionalInfo {
                        players_online: status.players.online,
                        players_max: status.players.max,
                        version: status.version.name,
                    });
                }

                let online =
                    container.state.unwrap().status.unwrap() == ContainerStateStatusEnum::RUNNING;

                server_list.push(ServerListEntry {
                    server: server.clone(),
                    online,
                    additional_info,
                });
            }

            let list_embed = CreateEmbed::new()
                .title("ℹ️ Servers")
                .color(0x04a5e5)
                .description("List of servers with info n' stuff!")
                .fields(create_server_list_fields(server_list));

            ctx.send(CreateReply::default().embed(list_embed)).await?;
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

/// Request to be whitelisted on a server
#[poise::command(slash_command)]
pub async fn request(
    ctx: Context<'_>,
    #[description = "ID of the server you want whitelisted in (use /list-servers to get a list of all server IDs!)"]
    #[autocomplete = "autocomplete_server_ids"]
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
