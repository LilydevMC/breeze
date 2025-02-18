use rcon::{Connection, Error};
use tokio::net::TcpStream;

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

pub async fn create_rcon_connection(
    address: &str,
    password: Option<String>,
) -> Result<Connection<TcpStream>, Error> {
    Connection::builder()
        .enable_minecraft_quirks(true)
        .connect(address, &password.unwrap_or("".to_string()))
        .await
}
