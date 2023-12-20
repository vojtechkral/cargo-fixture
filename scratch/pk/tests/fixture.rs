use std::time::Duration;
use std::{env, thread};

use cargo_fixture::FixtureClient;

use crate::shared::SharedData;

mod shared;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    dbg!(args);

    let mut fixture = FixtureClient::connect().await.unwrap();
    fixture.set_env_var("FOO", "bar").await.unwrap();
    fixture
        .set_value("abc", SharedData::new("foo"))
        .await
        .unwrap();
    fixture
        .set_extra_cargo_test_args(["--tests"])
        .await
        .unwrap();
    // fixture.set_extra_test_binary_args(["--nocapture"]).await.unwrap();
    dbg!(fixture.ready().await.unwrap());
}
