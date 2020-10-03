use serde::Deserialize;
#[derive(Debug, Deserialize)]
pub struct ConnectionDetails {
    pub database: String,
    pub username: String,
    pub password: String
}