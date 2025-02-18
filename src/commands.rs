use crate::{models::config::Server, Context, Error};
use bollard::{secret::ContainerStateStatusEnum, Docker};
use poise::{
    serenity_prelude::{
        ButtonStyle, ChannelId, CreateButton, CreateEmbed, CreateEmbedFooter, CreateMessage,
    },
    CreateReply,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerListEntry {
    pub server: Server,
    pub online: bool,
}

#[poise::command(slash_command, rename = "list-servers")]
pub async fn list_servers(ctx: Context<'_>) -> Result<(), Error> {
    let mut server_list: Vec<ServerListEntry> = vec![];

    match Docker::connect_with_defaults() {
        Ok(docker) => {
            for server in &ctx.data().config.servers {
                let container = docker.inspect_container(&server.container_id, None).await?;

                if container.state.unwrap().status.unwrap() == ContainerStateStatusEnum::RUNNING {
                    server_list.push(ServerListEntry {
                        server: server.clone(),
                        online: true,
                    });
                } else {
                    server_list.push(ServerListEntry {
                        server: server.clone(),
                        online: false,
                    });
                }
            }

            let description = server_list
                .iter()
                .map(|f| {
                    if f.online {
                        format!("🟢 **{}** `{}` - _Online_", f.server.name, f.server.id)
                    } else {
                        format!("🔴 **{}** `{}` - _Offline_", f.server.name, f.server.id)
                    }
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

// TODO: Event handling for button clicks
// Maybe use SQLite to store requests
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
            "<@{}> has requested to be whitelisted on server _{}_!\n\n**Minecraft Username:** `{}`\n**Server ID:** `{}`\n**Container ID:**: {}\n**Request ID:** `{}`",
            author.id, server.name, minecraft_username, server.id, server.container_id, request_id
        ))
		.footer(CreateEmbedFooter::new(ctx.created_at().to_string()));

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

    ctx.send(
        CreateReply::default()
            .ephemeral(true)
            .content("Sent whitelist request for server `SERVER NAME`!"),
    )
    .await?;

    Ok(())
}
