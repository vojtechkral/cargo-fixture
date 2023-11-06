use cargo_fixture::FixtureClient;

#[smol_potat::main]
async fn main() {
    let _fixture = FixtureClient::connect().await.unwrap();
    panic!("panic message");
}
