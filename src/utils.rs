use crate::Context;
use mc_query::rcon::RconClient;
use poise::serenity_prelude::AutocompleteChoice;

/// Returns true if the given Minecraft username is valid.
pub async fn validate_minecraft_username(username: &str) -> Result<bool, crate::Error> {
    match reqwest::get(&format!(
        "https://api.minecraftservices.com/minecraft/profile/lookup/name/{}",
        username
    ))
    .await
    {
        Ok(res) => Ok(res.status().is_success()),
        Err(err) => Err(err.into()),
    }
}

/// Creates a new [RconClient] and authenticates with the given password.
///
/// [RconClient](mc_query::rcon::RconClient)
pub async fn create_rcon_client(
    host: &str,
    port: u16,
    password: String,
) -> Result<RconClient, crate::Error> {
    let mut client = RconClient::new(host, port).await?;
    client.authenticate(&password).await?;
    Ok(client)
}

/// Autocompletes server IDs in commands based on the servers in the config.
pub async fn autocomplete_server_ids(
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
