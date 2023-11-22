use std::time::Duration;
use std::{env, thread};

use cargo_fixture::{self, set_fixture_data, Fixture};

use crate::shared::SharedData;

mod shared;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    dbg!(args);

    let mut fixture = Fixture::connect();
    fixture.set_env_var("FOO", "bar");
    // return;
    set_fixture_data!(fixture, "abc", SharedData::new("foo"));
    thread::sleep(Duration::from_millis(500));
    dbg!(fixture.ready());
    fixture.finalize();
}
