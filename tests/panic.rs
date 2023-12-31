use std::env;

use cargo_fixture::{with_fixture, TestClient};

mod common;
use common::{cargo_fixture, OutputExt as _};

#[test]
fn panic() {
    cargo_fixture().test("panic1").assert_error("panic message");
}

#[with_fixture]
#[smol_potat::test]
async fn panic1_callback(_client: TestClient) {
    let foo = env::var("FOO").unwrap();
    assert_eq!(foo, "bar");
}
