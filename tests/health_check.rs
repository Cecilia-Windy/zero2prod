use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::io::{sink, stdout};
use std::net::TcpListener;
use uuid::Uuid;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::run;
use zero2prod::telemetry::{get_subscriber, init_subscriber};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    // 根据TEST_LOG决定是否输出test日志
    match std::env::var("TEST_LOG") {
        Ok(var) if var.to_lowercase() == "true" => {
            let subscriber = get_subscriber(subscriber_name, default_filter_level, stdout);
            init_subscriber(subscriber);
        }
        _ => {
            let subscriber = get_subscriber(subscriber_name, default_filter_level, sink);
            init_subscriber(subscriber);
        }
    }
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_name: String,
    pub db_config: DatabaseSettings,
}

async fn spawn_app() -> TestApp {
    // 只有第一次调用会执行
    // 后续调用会直接跳过
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    let mut configuration = get_configuration().expect("Failed to read configuration.");
    let db_name = Uuid::new_v4().to_string();
    configuration.database.database_name = db_name.clone();
    let connection_pool = configure_database(&configuration.database).await;

    let server = run(listener, connection_pool.clone()).expect("Failed to run application");
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_pool: connection_pool,
        db_name,
        db_config: configuration.database,
    }
}

impl TestApp {
    async fn cleanup(&mut self) {
        // 关闭连接池以释放所有连接
        self.db_pool.close().await;

        // 连接到默认数据库以删除测试数据库
        let mut connection = PgConnection::connect(&self.db_config.connection_string_without_db())
            .await
            .expect("Failed to connect to Postgres for cleanup");

        // 终止所有连接到测试数据库的会话
        sqlx::query(
            r#"
            SELECT pg_terminate_backend(pid)
            FROM pg_stat_activity
            WHERE datname = $1 AND pid <> pg_backend_pid()
            "#,
        )
        .bind(&self.db_name)
        .execute(&mut connection)
        .await
        .expect("Failed to terminate existing connections");

        // 删除测试数据库
        connection
            .execute(format!(r#"DROP DATABASE "{}";"#, &self.db_name).as_str())
            .await
            .expect("Failed to drop database");
    }
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    // 创建数据库
    let mut connection = PgConnection::connect(&config.connection_string_without_db())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, &config.database_name).as_str())
        .await
        .expect("Failed to create database");

    // 迁移数据库
    let connection_pool = PgPool::connect(&config.connection_string())
        .await
        .expect("Failed to connect to Postgres");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    // 准备
    let mut app = spawn_app().await;
    let client = reqwest::Client::new();

    // 执行
    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    // 断言
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());

    app.cleanup().await;
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // 准备
    let mut app = spawn_app().await;
    let client = reqwest::Client::new();

    // 执行
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    // 断言
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");

    app.cleanup().await;
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    // 准备
    let mut app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // 执行
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // 断言
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }

    app.cleanup().await;
}
