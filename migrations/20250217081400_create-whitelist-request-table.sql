-- Add migration script here

CREATE TABLE IF NOT EXISTS whitelist_request (
    id VARCHAR(36) PRIMARY KEY,
    server_id TEXT NOT NULL,
    discord_id VARCHAR(19) NOT NULL,
    minecraft_username TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
