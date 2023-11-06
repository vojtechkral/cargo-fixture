use std::thread;
use std::time::Duration;

use cargo_fixture;

fn main() {
    let mut fixture = cargo_fixture::Client::new();
    fixture.set_env_var("FOO", "bar");
    thread::sleep(Duration::from_millis(500));
    let success = fixture.run_tests();
    dbg!(success);
    fixture.finalize();
}
