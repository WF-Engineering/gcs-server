use serde::Deserialize;
use yup_oauth2::ServiceAccountKey;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
  pub host: String,
  pub port: u32,
  pub service_account: String,
}

impl Config {
  pub fn sa_to_json(&self) -> serde_json::Result<ServiceAccountKey> {
    serde_json::from_str(&self.service_account)
  }
}

#[derive(Debug, Deserialize)]
pub struct Env {
  pub host: String,
  pub port: u32,
}

impl Env {
  pub fn to_address(&self) -> String {
    format!("{}:{}", self.host, self.port)
  }
}
