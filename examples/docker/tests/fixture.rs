use cargo_fixture::FixtureClient;
use dockertest::{
    waitfor::{MessageSource, MessageWait, WaitFor},
    DockerTest, Network, Source, TestBodySpecification,
};
use tokio_postgres::NoTls;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut fixture = FixtureClient::connect().await.unwrap();

    // Configure the postgres docker container
    let mut dt = DockerTest::new()
        .with_network(Network::Isolated)
        .with_default_source(Source::DockerHub);
    let password = "the-dancing-stones-are-restless";
    let mut postgres = TestBodySpecification::with_repository("postgres")
        .set_publish_all_ports(true)
        .set_wait_for(message_wait("ready to accept connections"));
    postgres
        .modify_env("POSTGRES_PASSWORD", &password)
        .modify_env("POSTGRES_DB", "example");
    dt.provide_container(postgres);

    // Spin up dockertest and run cargo tests as part of the closure.
    // Containers and other docker resources (network,..) will be released
    // after the call finishes.
    eprintln!("Preparing docker containers...");
    dt.run_async(|ops| async move {
        eprintln!("Containers ready...");
        let postgres = ops.handle("postgres");
        let (ip, port) = postgres.host_port(5432).unwrap();
        let uri = format!("postgres://postgres:{password}@{ip}:{port}/example");
        eprintln!("DB URI: {uri}");

        // Setup schema and generate some test content in the DB
        generate_example_table(&uri).await;

        fixture.set_env_var("POSTGRES_URI", uri).await.unwrap();

        // Tell the fixture we're ready to run tests.
        // This will return when cargo test call is complete.
        fixture.ready().await.unwrap();

        eprintln!("Cleaning up docker resources...");
    })
    .await;
}

fn message_wait(message: &str) -> Box<dyn WaitFor> {
    Box::new(MessageWait {
        message: message.to_string(),
        source: MessageSource::Stderr,
        timeout: 5,
    })
}

async fn generate_example_table(db_uri: &str) {
    eprintln!("Setting up DB schema...");

    let (client, connection) = tokio_postgres::connect(db_uri, NoTls).await.unwrap();

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    client
        .execute(
            "CREATE TABLE example (id SERIAL PRIMARY KEY, comment TEXT)",
            &[],
        )
        .await
        .unwrap();
    let insert = client
        .prepare("INSERT INTO example (comment) VALUES ($1)")
        .await
        .unwrap();
    for i in 1..=10 {
        client
            .execute(&insert, &[&format!("Test row n. {i}")])
            .await
            .unwrap();
    }
}
