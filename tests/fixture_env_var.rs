use cargo_fixture::FixtureClient;

#[smol_potat::main]
async fn main() {
    let mut fixture = FixtureClient::connect().await.unwrap();
    fixture.set_env_var("FOO", "foo").await.unwrap();
    fixture
        .set_env_vars([("BAR", "bar"), ("BAZ", "baz")])
        .await
        .unwrap();
    fixture.ready().await.unwrap();
}
