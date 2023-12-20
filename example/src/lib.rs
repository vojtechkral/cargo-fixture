use tokio_postgres::{Error, NoTls};

pub async fn count_example_rows(db_uri: &str) -> Result<i64, Error> {
    let (client, connection) = tokio_postgres::connect(db_uri, NoTls).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    client
        .query_one("SELECT count(*) FROM example", &[])
        .await?
        .try_get(0)
}
