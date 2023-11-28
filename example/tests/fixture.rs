use cargo_fixture::Fixture;
use dockertest::{
    waitfor::{MessageSource, MessageWait, WaitFor},
    DockerTest, Source, TestBodySpecification,
};

// TODO: comments

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let mut fixture = Fixture::connect().await.unwrap();

    let mut dt = DockerTest::new().with_default_source(Source::DockerHub);
    let password = "the-dancing-stones-are-restless";
    let mut postgres = TestBodySpecification::with_repository("postgres")
        .set_publish_all_ports(true)
        .set_wait_for(message_wait("ready to accept connections"));
    postgres.modify_env("POSTGRES_PASSWORD", &password);
    dt.provide_container(postgres);

    dt.run_async(|ops| async move {
        let postgres = ops.handle("postgres");
        let (ip, port) = postgres.host_port(5432).unwrap();
        let uri = format!("postgres://postgres:{password}@{ip}:{port}/postgres");
        eprintln!("DB URI: {uri}");

        fixture.set_env_var("POSTGRES_URI", uri).await.unwrap();
        fixture.ready().await.unwrap();
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
