// use mysql::Opts;
use crate::db_model::ConnectionDetails;

pub fn read_db_config() -> Result<ConnectionDetails, Box<dyn std::error::Error>> {
    let json = std::fs::read_to_string("db.json")?;
    // let c:Connection = serde_json::from_str(&json).unwrap();
    Ok(serde_json::from_str(&json)?)
}