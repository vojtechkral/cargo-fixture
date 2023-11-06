use cargo_fixture::FixtureClient;

pub mod common;
use common::fixture_hang;

#[smol_potat::main]
async fn main() {
    let mut fixture = FixtureClient::connect().await.unwrap();
    fixture.ready().await.unwrap();
    fixture_hang();
}
