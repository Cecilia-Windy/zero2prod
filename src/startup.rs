use crate::routes::*;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use actix_web::middleware::Logger;

pub fn run(listener: TcpListener, db_pool: PgPool) -> std::io::Result<Server> {
    let address = listener.local_addr()?;
    let db_pool = web::Data::new(db_pool);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .app_data(db_pool.clone())
    })
    .listen(listener)?
    .run();

    println!("Running on http://{address}");

    Ok(server)
}
