use std::time::Duration;
use std::{env, thread};

use cargo_fixture;

fn main() {
    let args = env::args().skip(1).collect::<Vec<_>>();
    dbg!(args);

    let mut fixture = cargo_fixture::Client::new();
    fixture.set_env_var("FOO", "bar");
    thread::sleep(Duration::from_millis(500));
    dbg!(fixture.ready());
    fixture.finalize();
}
