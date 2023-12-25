use cargo_fixture::{with_fixture, TestClient};

mod common;
use common::{cargo_fixture, KvExample, OutputExt as _};

#[test]
fn kv() {
    cargo_fixture().test("kv").assert_success();
}

#[with_fixture]
#[smol_potat::test]
async fn kv_callback(mut client: TestClient) {
    let example: KvExample = client.get_value("example").await.unwrap();
    assert_eq!(example.foo, "foo");
    assert_eq!(example.bar.to_string(), "127.0.0.1");
}
