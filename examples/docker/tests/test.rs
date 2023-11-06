use std::env;

use cargo_fixture::{with_fixture, TestClient};

#[with_fixture(serial)]
#[tokio::test]
async fn postgres_connect_basic(_client: TestClient) {
    let db_uri = env::var("POSTGRES_URI").unwrap();
    let count = cargo_fixture_example_docker::count_example_rows(&db_uri)
        .await
        .unwrap();

    assert_eq!(count, 10);
}

#[test]
fn some_other_test_no_fixture() {
    // This test will run even when you do just `cargo test`,
    // unlike the above test which will only run under `cargo fixture`.

    assert_eq!(1 + 1, 2);
}
