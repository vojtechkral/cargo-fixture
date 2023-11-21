use std::time::Duration;
use std::{env, thread};

use cargo_fixture::{self, set_fixture_data};

use crate::shared::SharedData;

mod shared;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    dbg!(args);

    loop {
        thread::sleep_ms(1000);
    }

    let mut fixture = cargo_fixture::Client::new();
    fixture.set_env_var("FOO", "bar");
    set_fixture_data!(fixture, "abc", SharedData::new("foo"));
    thread::sleep(Duration::from_millis(500));
    dbg!(fixture.ready());
    fixture.finalize();
}
