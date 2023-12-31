use cargo_fixture::{with_fixture, TestClient};

mod common;
use common::{cargo_fixture, OutputExt as _};

#[test]
fn hang_cleanup() {
    cargo_fixture().test("hang_cleanup").assert_success();
}

#[with_fixture]
#[smol_potat::test]
async fn hang_cleanup_callback(_client: TestClient) {}
