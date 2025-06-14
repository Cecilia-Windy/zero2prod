//! src/lib.rs

use actix_web::{App, HttpResponse, HttpServer, web};
use actix_web::dev::Server;
use std::net::TcpListener;

async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
}

pub fn run(listener: TcpListener) -> std::io::Result<Server> {
    let address = listener.local_addr().unwrap();

    let server = HttpServer::new(|| {
        App::new()
            .route("/health_check", web::get().to(health_check))
        })
        .listen(listener)?
        .run();

    println!("Running on http://{address}");

    Ok(server)
}
