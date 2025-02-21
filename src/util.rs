use mc_query::rcon::RconClient;

pub async fn validate_minecraft_username(username: &str) -> Result<bool, crate::Error> {
    match reqwest::get(&format!(
        "https://api.minecraftservices.com/minecraft/profile/lookup/name/{}",
        username
    ))
    .await
    {
        Ok(res) => Ok(res.status().is_success()),
        Err(_) => Ok(false),
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
