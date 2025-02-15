use crate::{config::Server, Context, Error};
use poise::{serenity_prelude::CreateEmbed, CreateReply};
use serde::{Deserialize, Serialize};

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
