//! tests/health_check.rs

use std::net::TcpListener;

fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");
    let address = listener.local_addr().unwrap();
    let server = zero2prod::run(listener).expect("Failed to bind address.");
    let _ = tokio::spawn(server);

    format!("http://{address}")
}

#[tokio::test]
async fn health_check_works() {
    //准备
    let address = spawn_app();
    let client = reqwest::Client::new();

    //执行
    let response = client
        .get(&format!("{}/health_check", &address))
        .send()
        .await
        .unwrap();

    //断言
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
