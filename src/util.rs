use rcon::{Connection, Error};
use tokio::net::TcpStream;

pub async fn create_rcon_connection(
    address: &str,
    password: Option<&str>,
) -> Result<Connection<TcpStream>, Error> {
    Connection::builder()
        .enable_minecraft_quirks(true)
        .connect(address, password.unwrap_or(""))
        .await
}
