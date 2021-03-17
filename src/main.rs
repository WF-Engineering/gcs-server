#[macro_use]
extern crate log;

mod api;
mod api_error;
mod config;

use std::io;

use actix_web::{middleware, web, App, HttpServer};
use config::{Config, Env};
use dotenv::dotenv;

#[actix_rt::main]
async fn main() -> io::Result<()> {
  dotenv().ok();
  env_logger::init();

  let env = envy::from_env::<Env>()
    .map_err(|err| error!("Deserilize env err: {:?}", err))
    .unwrap();

  let config = envy::from_env::<Config>()
    .map_err(|err| error!("Deserilize config err: {:?}", err))
    .unwrap();

  debug!("Running at {}", &env.to_address());

  HttpServer::new(move || {
    App::new()
      .data(config.clone())
      .wrap(middleware::Logger::default())
      .service(
        web::resource("/upload_object")
          .route(web::post().to(api::upload_object)),
      )
  })
  .bind(&env.to_address())?
  .run()
  .await
}
