use std::env;

use cargo_fixture::{with_fixture, TestClient};

use pk::add;

mod shared;
use shared::SharedData;

#[with_fixture]
#[tokio::test]
async fn it_works() {
    let result = add(2, 2);
    assert_eq!(result, 4);

    let foo = env::var("FOO").unwrap();
    assert_eq!(foo, "bar");

    let mut client = TestClient::connect(false).await.unwrap();
    let data = client.get_value::<SharedData>("abc").await.unwrap();
    assert_eq!(data.foo, "foo");
}

#[test]
fn it_works_2() {
    let result = add(2, 2);
    assert_eq!(result, 4);
}
