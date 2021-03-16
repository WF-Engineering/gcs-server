use std::io;

use actix_web::{HttpResponse, ResponseError};

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
  #[error("IO Error: {0:?}")]
  IoError(io::Error),

  #[error("Missing header key [{0:?}] from rquest")]
  MissingHeader(&'static str),

  #[error("Service account not found")]
  ServiceAccountNotFound,

  #[error("Failed to parse mime-type value")]
  MimeTypeParsingError,

  #[error("Failed to upload object cause: {0:?}")]
  UploadObjectError(google_storage1::Error),

  #[error("GCS response is not in 200 ..< 300, {0:?}")]
  NotSuccessResponse(hyper::client::Response),

  #[error("Failed to find filename by [{0:?}]")]
  MissingFilename(String),
}

impl From<hyper::client::Response> for ApiError {
  fn from(v: hyper::client::Response) -> Self {
    ApiError::NotSuccessResponse(v)
  }
}

impl From<google_storage1::Error> for ApiError {
  fn from(v: google_storage1::Error) -> Self {
    ApiError::UploadObjectError(v)
  }
}

impl From<io::Error> for ApiError {
  fn from(v: io::Error) -> Self {
    ApiError::IoError(v)
  }
}

impl ResponseError for ApiError {
  fn error_response(&self) -> HttpResponse {
    error!("{}", &self);

    match self {
      ApiError::IoError(_) => HttpResponse::InternalServerError(),
      ApiError::MissingHeader(_) => HttpResponse::BadRequest(),
      ApiError::ServiceAccountNotFound => HttpResponse::InternalServerError(),
      ApiError::MimeTypeParsingError => HttpResponse::BadRequest(),
      ApiError::UploadObjectError(_) => HttpResponse::InternalServerError(),
      ApiError::NotSuccessResponse(_) => HttpResponse::InternalServerError(),
      ApiError::MissingFilename(_) => HttpResponse::InternalServerError(),
    }
    .body(self.to_string())
  }
}
