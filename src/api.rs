use std::fs;
use std::io::prelude::*;
use std::path::Path;

use actix_web::{web, HttpRequest, HttpResponse};
use google_storage1::{Object, Storage};
use hyper::net::HttpsConnector;
use hyper_rustls::TlsClient;
use yup_oauth2 as oauth2;

use super::api_error::ApiError;
use super::config::Config;

pub async fn upload_object(
  request: HttpRequest,
  payload: web::Bytes,
  config: web::Data<Config>,
) -> Result<HttpResponse, ApiError> {
  let name = request
    .headers()
    .get("Name")
    .and_then(|h| h.to_str().ok())
    .ok_or(ApiError::MissingHeader("Name"))?;

  let bucket = request
    .headers()
    .get("Bucket")
    .and_then(|h| h.to_str().ok())
    .ok_or(ApiError::MissingHeader("Bucket"))?;

  let mime_type = request
    .headers()
    .get("Mime-Type")
    .and_then(|h| h.to_str().ok())
    .ok_or(ApiError::MissingHeader("Mime-Type"))?;

  let saved_path = Path::new(name)
    .file_name()
    .ok_or(ApiError::MissingFilename(name.to_string()))?;

  let mut file = fs::File::create(saved_path)?;
  file.write_all(&payload)?;

  let client =
    hyper::Client::with_connector(HttpsConnector::new(TlsClient::new()));

  let client_secret = config.sa_to_json().map_err(|err| {
    error!("err: {:?}", err);
    ApiError::ServiceAccountNotFound
  })?;

  let authenticator = oauth2::ServiceAccountAccess::new(client_secret, client);
  let client =
    hyper::Client::with_connector(HttpsConnector::new(TlsClient::new()));

  let hub = Storage::new(client, authenticator);

  let object = Object::default();
  let (response, object) =
    hub.objects().insert(object, bucket).name(name).upload(
      fs::File::open(saved_path)?,
      mime_type
        .parse()
        .map_err(|_| ApiError::MimeTypeParsingError)?,
    )?;

  fs::remove_file(saved_path)?;

  if response.status.is_success() {
    Ok(HttpResponse::Ok().json(object))
  } else {
    Err(ApiError::NotSuccessResponse(response))
  }
}
