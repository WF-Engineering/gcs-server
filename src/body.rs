use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DeleteObject {
  pub object: String,
  pub bucket: String,
}
