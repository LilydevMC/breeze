use crate::{Context, Error, models::config::Server, utils::autocomplete_server_ids};
use bollard::{Docker, secret::ContainerStateStatusEnum};
use poise::{CreateReply, serenity_prelude as serenity};
use serde::{Deserialize, Serialize};
use serenity::{CreateEmbed, CreateEmbedFooter};

pub mod whitelist;

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

#[poise::command(slash_command, subcommands("list", "players"))]
pub async fn server(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// List all servers with their status and additional info if available
#[poise::command(slash_command)]
async fn list(ctx: Context<'_>) -> Result<(), Error> {
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
                .fields(create_server_list_fields(server_list))
                .footer(CreateEmbedFooter::new(
                    "Looking for a list of players? Use `/server players`!",
                ));

            ctx.send(CreateReply::default().embed(list_embed)).await?;
        }
        Err(_) => {
            ctx.send(CreateReply::default().content("Failed to connect to Docker daemon."))
                .await?;
        }
    }

    Ok(())
}

/// Get a list of players on a server
#[poise::command(slash_command)]
async fn players(
    ctx: Context<'_>,
    #[description = "ID of the target server"]
    #[autocomplete = "autocomplete_server_ids"]
    server_id: String,
) -> Result<(), Error> {
    let config = &ctx.data().config;

    let server = config
        .servers
        .iter()
        .find(|server| server.id == server_id)
        .ok_or(format!("Server with ID `{}` not found", server_id))?;

    let query = match mc_query::status(&server.address, server.query_port).await {
        Ok(query) => query,
        Err(_) => {
            ctx.send(
                CreateReply::default()
                    .content("Failed to query server!")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    };

    let players = match query.players.sample {
        Some(players) => players,
        None => {
            ctx.send(
                CreateReply::default()
                    .content("No players found!")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    };

    // This looks kinda ugly but I'm not sure how to format it better
    let player_list: String = players
        .into_iter()
        .map(|p| format!("- {}", p.name))
        .collect::<Vec<String>>()
        .join("\n");

    let embed_description = format!(
        "{} players online on _**{}**_!\n\n{}",
        query.players.online, server.name, player_list
    );

    ctx.send(
        CreateReply::default().embed(
            CreateEmbed::new()
                .title("🫂 Online players")
                .description(embed_description)
                .color(0x04a5e5),
        ),
    )
    .await?;

    Ok(())
}
