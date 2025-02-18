use bollard::secret::ContainerStateStatusEnum;
use poise::serenity_prelude as serenity;
use serenity::{CacheHttp, Context, CreateEmbed, CreateInteractionResponseFollowup, FullEvent};

use crate::{models::database::WhitelistRequest, util, Data, Error};

pub async fn event_handler(
    ctx: &Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let FullEvent::InteractionCreate { interaction } = event {
        if let Some(interaction_component) = interaction.as_message_component() {
            let id = interaction_component.data.custom_id.clone();

            println!("Interaction ID: {}", id);

            if id.starts_with("wlreq-") {
                let config = &data.config;
                let guild_id = interaction_component.guild_id.unwrap();

                let interaction_author = &interaction_component.user;

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

                if interaction_component
                    .user
                    .member
                    .as_deref()
                    .unwrap()
                    .permissions
                    .unwrap()
                    .administrator()
                {
                    is_user_authorized = true;
                }

                if !is_user_authorized {
                    interaction_component
                        .create_response(
                            ctx.http(),
                            serenity::CreateInteractionResponse::Acknowledge,
                        )
                        .await?;
                    interaction_component
                        .create_followup(
                            ctx.http(),
                            CreateInteractionResponseFollowup::new()
                                .add_embed(
                                    CreateEmbed::new()
                                        .title(":x: Error: Unauthorized")
                                        .color(0xd20f39)
                                        .description(
                                            "You are not authorized to perform this action",
                                        ),
                                )
                                .ephemeral(true),
                        )
                        .await?;

                    return Ok(());
                }

                // if interaction_author.has_role(cache_http, guild_id, role)

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

                    let server = &config
                        .servers
                        .iter()
                        .find(|s| s.id == request_info.server_id)
                        .unwrap();

                    let container_status = bollard::Docker::connect_with_local_defaults()
                        .unwrap()
                        .inspect_container(&server.container_id, None)
                        .await?
                        .state
                        .unwrap()
                        .status
                        .unwrap();

                    if id.contains("approve") {
                        if container_status == ContainerStateStatusEnum::RUNNING {
                            let server_addr = format!("{}:{}", server.address, server.rcon_port);

                            let mut conn = util::create_rcon_connection(
                                &server_addr,
                                server.rcon_password.clone(),
                            )
                            .await?;

                            conn.cmd(&format!(
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

                            println!("Approved request {}", request_id);

                            interaction_component
                                .create_response(
                                    ctx.http(),
                                    serenity::CreateInteractionResponse::Acknowledge,
                                )
                                .await?;
                            interaction_component
										.create_followup(
											ctx.http(),
											CreateInteractionResponseFollowup::new()
												.add_embed(
													CreateEmbed::new()
														.title(":white_check_mark: Whitelist request approved!")
														.color(0x40a02b)
														.description(
															format!("Whitelist request approved for <@{}>!", request_info.discord_id),
														),
												)
												.ephemeral(true),
										)
										.await?;
                        } else {
                            interaction_component
                                .create_response(
                                    ctx.http(),
                                    serenity::CreateInteractionResponse::Acknowledge,
                                )
                                .await?;
                            interaction_component
                                .create_followup(
                                    ctx.http(),
                                    CreateInteractionResponseFollowup::new()
                                        .add_embed(
                                            CreateEmbed::new()
                                                .title(":x: Error: Server not running")
                                                .color(0xd20f39)
                                                .description("Server container is not running"),
                                        )
                                        .ephemeral(true),
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

                        interaction_component
                            .create_response(
                                ctx.http(),
                                serenity::CreateInteractionResponse::Acknowledge,
                            )
                            .await?;
                        interaction_component
                            .create_followup(
                                ctx.http(),
                                CreateInteractionResponseFollowup::new()
                                    .add_embed(
                                        CreateEmbed::new()
                                            .title(":x: Whitelist request denied!")
                                            .color(0xd20f39)
                                            .description(format!(
                                                "Whitelist request denied for <@{}>!",
                                                request_info.discord_id
                                            )),
                                    )
                                    .ephemeral(true),
                            )
                            .await?;

                        println!("Denied request {}", request_id);
                    }
                } else {
                    interaction_component
                        .create_response(
                            ctx.http(),
                            serenity::CreateInteractionResponse::Acknowledge,
                        )
                        .await?;
                    interaction_component
                        .create_followup(
                            ctx.http(),
                            CreateInteractionResponseFollowup::new()
                                .add_embed(
                                    CreateEmbed::new()
                                        .title(":x: Error: Whitelist request not found")
                                        .color(0xd20f39)
                                        .description("Whitelist request not found in the database"),
                                )
                                .ephemeral(true),
                        )
                        .await?;
                }
            }
        }
    }

    Ok(())
}
