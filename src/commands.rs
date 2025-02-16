use crate::{config::Server, Context, Error};
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

    for server in &ctx.data().config.servers {
        let server_addr = format!("{}:{}", &server.address, &server.rcon_port);
        match crate::util::create_rcon_connection(&server_addr, server.rcon_password.as_deref())
            .await
        {
            Ok(mut _conn) => {
                // TODO: Get amount of players currently online and display it in embed
                server_list.push(ServerListEntry {
                    server: server.clone(),
                    online: true,
                });
            }
            Err(_) => {
                server_list.push(ServerListEntry {
                    server: server.clone(),
                    online: false,
                });
            }
        };
    }
    let description = server_list
        .iter()
        .map(|f| {
            if f.online {
                format!("🟢 **{}**: Online", f.server.name)
            } else {
                format!("🔴 **{}**: Offline", f.server.name)
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    ctx.send(
        CreateReply::default().embed(
            CreateEmbed::new()
                .title("ℹ️ Servers")
                .description(description),
        ),
    )
    .await?;
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
    let author = ctx.author();

    let request_id = Uuid::new_v4();

    let server = config
        .servers
        .iter()
        .find(|server| server.id == server_id)
        .ok_or("Server not found")?; // TODO: Global custom embed for errors like this

    let approve_button = CreateButton::new(format!("approve-{}", request_id))
        .label("Approve")
        .style(ButtonStyle::Success);

    let deny_button = CreateButton::new(format!("deny-{}", request_id))
        .label("Deny")
        .style(ButtonStyle::Danger);

    let request_embed = CreateEmbed::new()
        .title(":bell: Whitelist Request")
		.color(0xdf8e1d)
        .description(format!(
            "<@{}> has requested to be whitelisted on server _{}_!\n\n**Minecraft Username:** `{}`\n**Server ID:** `{}`\n**Request ID:** `{}`",
            author.id, server.name, minecraft_username, server.id, request_id
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
