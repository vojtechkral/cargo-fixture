use std::{thread, time::Duration};

use cargo_fixture::FixtureClient;

#[smol_potat::main]
async fn main() {
    let mut fixture = FixtureClient::connect().await.unwrap();
    fixture.ready().await.unwrap();

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
