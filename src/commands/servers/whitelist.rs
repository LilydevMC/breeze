use crate::{Context, Error, utils::autocomplete_server_ids};
use chrono::Utc;
use poise::{CreateReply, serenity_prelude as serenity};
use serenity::{
    ButtonStyle, ChannelId, CreateButton, CreateEmbed, CreateEmbedFooter, CreateMessage,
};
use uuid::Uuid;

#[poise::command(slash_command, subcommands("request"))]
pub async fn whitelist(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Request to be whitelisted on a server
#[poise::command(slash_command)]
async fn request(
    ctx: Context<'_>,
    #[description = "ID of the target server"]
    #[autocomplete = "autocomplete_server_ids"]
    server_id: String,
    #[description = "Your Minecraft username"] minecraft_username: String,
) -> Result<(), Error> {
    let config = &ctx.data().config;

    if !crate::utils::validate_minecraft_username(&minecraft_username).await? {
        return Err(
            format!(
				"Invalid Minecraft username `{minecraft_username}`. Please make sure you've entered it correctly."
			).into(),
        );
    }

    let author = ctx.author();
    let author_id = author.id.to_string();

    let request_id = Uuid::new_v4();
    let request_id_s = request_id.to_string();

    let server = config
        .servers
        .iter()
        .find(|server| server.id == server_id)
        .ok_or(format!("Server with ID `{}` not found", server_id))?;

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
