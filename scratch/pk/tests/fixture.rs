use std::time::Duration;
use std::{env, thread};

use cargo_fixture::{self, set_fixture_data, Fixture, FixtureClient};

use crate::shared::SharedData;

mod shared;

#[tokio::main]
async fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    dbg!(args);

    // let mut fixture = Fixture::connect().unwrap();
    // fixture.set_env_var("FOO", "bar").unwrap();
    // set_fixture_data!(fixture, "abc", SharedData::new("foo")).unwrap();
    // fixture.set_additional_cargo_test_args(["--help"]);
    // fixture.set_additional_harness_args(["--help"]);
    // thread::sleep(Duration::from_millis(500));
    // dbg!(fixture.ready().unwrap());

    let client = FixtureClient::connect();
}
