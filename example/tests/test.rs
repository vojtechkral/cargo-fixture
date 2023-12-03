use std::env;

use cargo_fixture::with_fixture;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
#[with_fixture]
async fn postgres_connect_basic(#[env] postgres_uri: String) {
    // async fn postgres_connect_basic() {
    // let postgres_uri = env::var("POSTGRES_URI").unwrap();

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&postgres_uri)
        .await
        .unwrap();

    let (num,): (i64,) = sqlx::query_as("SELECT $1")
        .bind(42_i64)
        .fetch_one(&pool)
        .await
        .unwrap();

    assert_eq!(num, 42);
}
