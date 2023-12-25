use std::net::IpAddr;

use cargo_fixture::FixtureClient;

mod common;
use common::KvExample;

#[smol_potat::main]
async fn main() {
    let mut fixture = FixtureClient::connect().await.unwrap();

    let example_value = KvExample {
        foo: "foo".to_string(),
        bar: IpAddr::from([127, 0, 0, 1]),
    };
    fixture.set_value("example", example_value).await.unwrap();

    fixture.ready().await.unwrap();
}
