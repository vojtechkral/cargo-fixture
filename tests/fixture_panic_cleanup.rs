use cargo_fixture::FixtureClient;

#[smol_potat::main]
async fn main() {
    let mut fixture = FixtureClient::connect().await.unwrap();
    fixture.ready().await.unwrap();
    panic!("panic message");
}
