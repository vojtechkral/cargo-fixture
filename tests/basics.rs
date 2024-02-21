use std::env;

use cargo_fixture::{with_fixture, TestClient};

pub mod common;
use common::{cargo_fixture, confirm_callback_ran};

use crate::common::KvExample;

// Callbacks:
// tests named `foo_callback`` are to be called by the fixture from test `foo`
// It doesn't have to necessarily exist, but when it does and the test is expected
// to pass, we verify the callback ran by having it write an ID passed in a env var
// to a file based on the test name. We then verify the ID in the file in assert_success().

#[test]
fn env_var() {
    cargo_fixture()
        .run_test("env_var")
        .output()
        .assert_success();
}

#[with_fixture]
#[smol_potat::test]
async fn env_var_callback(_client: TestClient) {
    let foo = env::var("FOO").unwrap();
    assert_eq!(foo, "bar");
    confirm_callback_ran("env_var");
}

#[test]
fn failing_test() {
    cargo_fixture()
        .run_test("failing_test")
        .output()
        .assert_error("thread 'failing_test_callback' panicked");
}

#[with_fixture]
#[smol_potat::test]
async fn failing_test_callback(_client: TestClient) {
    panic!();
}

#[test]
fn kv() {
    cargo_fixture().run_test("kv").output().assert_success();
}

#[with_fixture]
#[smol_potat::test]
async fn kv_callback(mut client: TestClient) {
    let example: KvExample = client.get_value("example").await.unwrap();
    assert_eq!(example.foo, "foo");
    assert_eq!(example.bar.to_string(), "127.0.0.1");
    confirm_callback_ran("kv");
}

#[test]
fn early_exit() {
    cargo_fixture()
        .run_test("early_exit")
        .output()
        .assert_error("fixture program exited without connecting to fixture");
}

#[test]
fn panic() {
    cargo_fixture()
        .run_test("panic_init")
        .output()
        .assert_error("panic message");

    cargo_fixture()
        .run_test("panic_connected")
        .output()
        .assert_error("panic message");

    cargo_fixture()
        .run_test("panic_cleanup")
        .output()
        .assert_error("panic message");
}

#[cfg(unix)]
#[test]
fn hang() {
    use common::hang_file;

    let hang_file_cleanup = hang_file("hang_cleanup");
    cargo_fixture()
        .env("HANG_FILE", hang_file_cleanup.path())
        .check_socket_exists(true)
        .run_test("hang_cleanup")
        .wait_fixture_hang(hang_file_cleanup.path())
        .kill_fixture()
        .assert_error("fixture program failed: killed by a signal");
}
