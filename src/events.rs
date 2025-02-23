use bollard::{Docker, secret::ContainerStateStatusEnum};
use poise::serenity_prelude as serenity;
use serenity::{
    CacheHttp, Context, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateMessage, FullEvent, UserId,
};

use crate::{Data, Error, models::database::WhitelistRequest, util};

async fn create_interaction_followup(
    ctx: &Context,
    component_interaction: &serenity::ComponentInteraction,
    title: &str,
    description: &str,
    color: u32,
    ephemeral: bool,
) -> Result<(), Error> {
    let http = ctx.http();
    component_interaction
        .create_response(http, CreateInteractionResponse::Acknowledge)
        .await?;
    component_interaction
        .create_followup(
            http,
            CreateInteractionResponseFollowup::new()
                .add_embed(
                    CreateEmbed::new()
                        .title(title)
                        .description(description)
                        .color(color),
                )
                .ephemeral(ephemeral),
        )
        .await?;

    Ok(())
}

async fn create_error_followup(
    ctx: &Context,
    component_interaction: &serenity::ComponentInteraction,
    error_title: &str,
    error_description: &str,
) -> Result<(), Error> {
    create_interaction_followup(
        ctx,
        component_interaction,
        &format!(":x: Error: {error_title}"),
        error_description,
        0xd20f39,
        true,
    )
    .await?;

    Ok(())
}

fn create_dm_footer(guild_name: String, guild_icon: Option<String>) -> CreateEmbedFooter {
    CreateEmbedFooter::new(format!("From server {guild_name}")).icon_url(
        guild_icon.unwrap_or("https://files.jadelily.dev/ZabtFsxYPYvgO2bpKry3.png".to_string()),
    )
}

pub async fn event_handler(
    ctx: &Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let FullEvent::InteractionCreate { interaction } = event {
        if let Some(component_interaction) = interaction.as_message_component() {
            let id = component_interaction.data.custom_id.clone();

            println!("Interaction ID: {}", id);

            if id.starts_with("wlreq-") {
                let config = &data.config;
                let guild_id = match component_interaction.guild_id {
                    Some(guild_id) => guild_id,
                    None => {
                        return Err(anyhow::anyhow!("Guild ID not found in interaction").into());
                    }
                };

                // let guild_icon = guild_id.get_preview(&ctx).await?.icon;
                let guild_icon = guild_id.to_partial_guild(&ctx).await?.icon_url();
                let guild_name = guild_id.name(ctx).unwrap();

                let interaction_author = &component_interaction.user;

                let mut is_user_authorized = false;

                for role_id in config.whitelist.allowed_roles.clone() {
                    let has_role = interaction_author
                        .has_role(&ctx.http, guild_id, role_id)
                        .await?;

                    if has_role {
                        is_user_authorized = true;
                        break;
                    }
                }

                if let Some(member) = interaction_author.member.as_deref() {
                    if let Some(permissions) = member.permissions {
                        if config.whitelist.allow_admin && permissions.administrator() {
                            is_user_authorized = true;
                        }
                    }
                };

                if !is_user_authorized {
                    create_error_followup(
                        ctx,
                        component_interaction,
                        "Unauthorized!",
                        "You are not authorized to perform this action",
                    )
                    .await?;

                    return Ok(());
                }

                let request_id = match id.contains("approve") {
                    true => id.replace("wlreq-approve-", ""),
                    false => id.replace("wlreq-deny-", ""),
                };

                let exists = sqlx::query!(
                    "SELECT EXISTS(SELECT 1 FROM whitelist_request WHERE id = ?) as 'exists'",
                    request_id
                )
                .fetch_one(&data.db)
                .await?
                .exists
                    > 0;

                if exists {
                    let request_info: WhitelistRequest = sqlx::query_as!(
                        WhitelistRequest,
                        "
							SELECT *
							FROM whitelist_request
							WHERE id = ?
						",
                        request_id
                    )
                    .fetch_one(&data.db)
                    .await?;

                    let server = match config
                        .servers
                        .iter()
                        .find(|s| s.id == request_info.server_id)
                    {
                        Some(server) => server,
                        None => {
                            create_error_followup(
                                ctx,
                                component_interaction,
                                "Server not found!",
                                &format!(
                                    "Server with the ID `{}` not found",
                                    request_info.server_id
                                ),
                            )
                            .await?;

                            return Err(anyhow::anyhow!("Server not found").into());
                        }
                    };

                    if server.container_id.is_empty() {
                        create_error_followup(
                            ctx,
                            component_interaction,
                            "Container ID not found!",
                            &format!("Container ID not found for server `{}`. Please add one in your `config.toml` file.", server.id),
                        )
                        .await?;

                        return Err(anyhow::anyhow!("Container ID not found").into());
                    }

                    let container_status = match Docker::connect_with_defaults()?
                        .inspect_container(&server.container_id, None)
                        .await?
                        .state
                    {
                        Some(state) => match state.status {
                            Some(status) => status,
                            None => {
                                create_error_followup(
                                    ctx,
                                    component_interaction,
                                    "Failed to get server status!",
                                    &format!(
										"Failed to get server status from Docker container with the ID `{}`!",
										server.container_id
									),
                                )
                                .await?;

                                return Err(anyhow::anyhow!(
                                    "Failed to get server status from Docker container"
                                )
                                .into());
                            }
                        },
                        None => {
                            create_error_followup(
                                ctx,
                                component_interaction,
                                "Failed to get server state!",
                                &format!(
                                    "Failed to get server state from Docker container with the ID `{}`",
                                    server.container_id
                                ),
                            )
                            .await?;

                            return Err(anyhow::anyhow!(
                                "Failed to get server state from Docker container"
                            )
                            .into());
                        }
                    };

                    let requester_id = request_info.discord_id.parse::<u64>().unwrap();
                    let user = UserId::new(requester_id);

                    if id.contains("approve") {
                        if container_status == ContainerStateStatusEnum::RUNNING {
                            let mut rcon_client = util::create_rcon_client(
                                &server.address,
                                server.rcon_port,
                                server.rcon_password.clone(),
                            )
                            .await?;

                            rcon_client
                                .run_command(&format!(
                                    "whitelist add {}",
                                    request_info.minecraft_username
                                ))
                                .await?;

                            sqlx::query!(
                                "
									DELETE FROM whitelist_request
									WHERE id = ?
								",
                                request_id
                            )
                            .execute(&data.db)
                            .await?;

                            if config.whitelist.send_approval_dm {
                                if let Err(error) = user
									.dm(
										ctx.http(),
										CreateMessage::new().add_embed(
											CreateEmbed::new()
												.title("✅ Your whitelist request has been approved!")
												.description(
													format!(
														"Your whitelist request for the server _**{}**_ has been approved!\n\n**Server ID:** `{}`\n**Minecraft Username:** `{}`",
														server.name, server.id, request_info.minecraft_username
													)
												)
												.footer(
													create_dm_footer(guild_name, guild_icon)
												)
												.color(0x40a02b),
										),
									)
									.await
								{
									println!("Error sending DM: {:?}", error);
								};
                            }

                            create_interaction_followup(
                                ctx,
                                component_interaction,
                                ":white_check_mark: Whitelist request approved!",
                                &format!(
                                    "Whitelist request approved for <@{}>!",
                                    request_info.discord_id
                                ),
                                0x40a02b,
                                true,
                            )
                            .await?;
                        } else {
                            create_error_followup(
                                ctx,
                                component_interaction,
                                "Server not running!",
                                "Server container is not running",
                            )
                            .await?;
                        }
                    } else if id.contains("deny") {
                        sqlx::query!(
                            "
								DELETE FROM whitelist_request
								WHERE id = ?
							",
                            request_id
                        )
                        .execute(&data.db)
                        .await?;

                        if config.whitelist.send_denial_dm {
                            if let Err(error) = user
								.dm(
									ctx.http(),
									CreateMessage::new().add_embed(
										CreateEmbed::new()
											.title("❌ Your whitelist request has been denied.")
											.description(
												format!(
													"Your whitelist request for the server _**{}**_ has been denied.\n\n**Server ID:** `{}`\n**Minecraft Username:** `{}`",
													server.name, server.id, request_info.minecraft_username
												)
											)
											.footer(
												create_dm_footer(guild_name, guild_icon)
											)
											.color(0xd20f39),
									),
								)
								.await
							{
								println!("Error sending DM: {:?}", error);
							};
                        }

                        create_interaction_followup(
                            ctx,
                            component_interaction,
                            ":x: Whitelist request denied!",
                            &format!(
                                "Whitelist request denied for <@{}>!",
                                request_info.discord_id
                            ),
                            0xd20f39,
                            true,
                        )
                        .await?;

                        println!("Denied request {}", request_id);
                    }
                } else {
                    create_error_followup(
                        ctx,
                        component_interaction,
                        "Whitelist request not found!",
                        &format!("Whitelist request `{}` not found in database", request_id),
                    )
                    .await?;
                }
            }
        }
    }

    Ok(())
}
