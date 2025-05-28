use std::fs;
use std::path::Path;
use actix_multipart::Multipart;
use actix_web::{web, HttpRequest, HttpResponse};
use futures::StreamExt;
use google_storage1::{api::Object, Storage};
use hyper::{Client, body::Body, Response};
use hyper_rustls::HttpsConnectorBuilder;
use yup_oauth2::{ServiceAccountKey, ServiceAccountAuthenticator};
use crate::api_error::ApiError;
use crate::body;
use crate::config::Config;

pub async fn upload_object(
    request: HttpRequest,
    mut payload: Multipart,
    config: web::Data<Config>,
) -> Result<HttpResponse, ApiError> {
    let name = request.headers().get("Name")
        .and_then(|h| h.to_str().ok())
        .ok_or(ApiError::MissingHeader("Name"))?;
    let bucket = request.headers().get("Bucket")
        .and_then(|h| h.to_str().ok())
        .ok_or(ApiError::MissingHeader("Bucket"))?;
    let mime_type = request.headers().get("Mime-Type")
        .and_then(|h| h.to_str().ok())
        .ok_or(ApiError::MissingHeader("Mime-Type"))?;

    let filepath = Path::new(name)
        .file_name()
        .and_then(|s| s.to_str())
        .map(sanitize_filename::sanitize)
        .ok_or_else(|| ApiError::MissingFilename(name.to_string()))?;

    // 寫檔案
    while let Some(field) = payload.next().await {
        let mut field = field?;
        let filepath = filepath.clone();
        let mut f = web::block(move || fs::File::create(filepath)).await??;

        while let Some(chunk) = field.next().await {
            let data = chunk?;
            f = web::block(move || {
                use std::io::Write;
                f.write_all(&data).map(|_| f)
            }).await??;
        }
    }

    // HYPER client & yup-oauth2 Authenticator
    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();
    let client = Client::builder().build::<_, hyper::Body>(https);

    let sa_key = config.sa_to_json().map_err(|_| ApiError::ServiceAccountNotFound)?;
    let auth = ServiceAccountAuthenticator::builder(sa_key)
        .build()
        .await
        .map_err(|_| ApiError::ServiceAccountNotFound)?;

    let hub = Storage::new(client, auth);

    // 寫入 GCS
    let object = Object::default();
    let (response, object) = hub.objects().insert(object, bucket)
        .name(name)
        .upload(
            fs::File::open(filepath.clone())?,
            mime_type.parse().map_err(|_| ApiError::MimeTypeParsingError)?,
        ).await?;

    // 清除暫存檔案
    web::block(move || fs::remove_file(filepath)).await??;

    if response.status().is_success() {
        Ok(HttpResponse::Ok().json(object))
    } else {
        Err(ApiError::NotSuccessResponse(response))
    }
}

pub async fn delete_object(
    config: web::Data<Config>,
    payload: web::Json<body::DeleteObject>,
) -> Result<HttpResponse, ApiError> {
    let https = HttpsConnectorBuilder::new()
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();
    let client = Client::builder().build::<_, Body>(https);

    let sa_key = config.sa_to_json().map_err(|_| ApiError::ServiceAccountNotFound)?;
    let auth = ServiceAccountAuthenticator::builder(sa_key)
        .build()
        .await
        .map_err(|_| ApiError::ServiceAccountNotFound)?;

    let hub = Storage::new(client, auth);

    let payload = payload.into_inner();
    let bucket = payload.bucket;
    let object = payload.object;

    let result = hub.objects().delete(&bucket, &object).doit().await;

    match result {
        Ok(response) if response.status().is_success() => Ok(HttpResponse::NoContent().finish()),
        Ok(response) => Err(ApiError::NotSuccessResponse(response)),
        Err(err) => {
            error!("delete_object err: {:?}", err);
            Err(ApiError::DeleteObjectFailed)
        }
    }
}
