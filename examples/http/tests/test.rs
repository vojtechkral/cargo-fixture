use std::env;

use cargo_fixture::{with_fixture, TestClient};

#[with_fixture]
#[tokio::test]
async fn http_request(_client: TestClient) {
    let port = env::var("HTTP_PORT").unwrap().parse::<u16>().unwrap();
    let response = cargo_fixture_example_http::request(port).await.unwrap();

    assert_eq!(response, "OK");
}

#[test]
fn some_other_test_no_fixture() {
    // This test will run even when you do just `cargo test`,
    // unlike the above test which will only run under `cargo fixture`.

    assert_eq!(1 + 1, 2);
}
