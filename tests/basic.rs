mod common;

use std::env;

use cargo_fixture::{with_fixture, TestClient};
use common::OutputExt;

use crate::common::cargo_fixture;

#[test]
fn basic() {
    cargo_fixture("basic").assert_success();
}

#[with_fixture]
#[smol_potat::test]
async fn basic_callback(_client: TestClient) {
    let foo = env::var("FOO").unwrap();
    assert_eq!(foo, "bar");
}
