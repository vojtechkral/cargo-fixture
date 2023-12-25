use std::env;

use cargo_fixture::{with_fixture, TestClient};

mod common;
use common::{cargo_fixture, OutputExt as _};

#[test]
fn basic() {
    cargo_fixture().test("basic").assert_success();
}

#[with_fixture]
#[smol_potat::test]
async fn basic_callback(_client: TestClient) {
    let foo = env::var("FOO").unwrap();
    assert_eq!(foo, "bar");
}
