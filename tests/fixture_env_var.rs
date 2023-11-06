use cargo_fixture::FixtureClient;

#[smol_potat::main]
async fn main() {
    let mut fixture = FixtureClient::connect().await.unwrap();
    fixture.set_env_var("FOO", "bar").await.unwrap();
    fixture.ready().await.unwrap();
}
