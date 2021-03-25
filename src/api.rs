use std::fs;
use std::io::prelude::*;
use std::path::Path;

use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use futures::StreamExt;
use google_storage1::{Object, Storage};
use hyper::net::HttpsConnector;
use hyper_rustls::TlsClient;
use yup_oauth2 as oauth2;

use super::api_error::ApiError;
use super::config::Config;

pub async fn upload_object(
  request: HttpRequest,
  mut payload: Multipart,
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

  let filepath = Path::new(name)
    .file_name()
    .and_then(|s| s.to_str())
    .map(sanitize_filename::sanitize)
    .ok_or_else(|| ApiError::MissingFilename(name.to_string()))?;

  while let Some(field) = payload.next().await {
    let mut field = field?;

    let filepath = filepath.clone();

    // File::create is blocking operation, use threadpool
    let mut f = web::block(|| fs::File::create(filepath)).await?;

    while let Some(chunk) = field.next().await {
      let data = chunk?;

      // filesystem operations are blocking, we have to use threadpool
      f = web::block(move || f.write_all(&data).map(|_| f)).await?
    }
  }

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
      fs::File::open(filepath.clone())?,
      mime_type
        .parse()
        .map_err(|_| ApiError::MimeTypeParsingError)?,
    )?;

  // filesystem operations are blocking, we have to use threadpool
  web::block(move || fs::remove_file(filepath)).await?;

  if response.status.is_success() {
    Ok(HttpResponse::Ok().json(object))
  } else {
    Err(ApiError::NotSuccessResponse(response))
  }
}
