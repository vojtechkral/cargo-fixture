use std::env;

use cargo_fixture::{get_fixture_data, with_fixture};

use pk::add;

mod shared;
use shared::SharedData;

#[with_fixture]
#[test]
fn it_works() {
    let result = add(2, 2);
    assert_eq!(result, 4);

    let foo = env::var("FOO").unwrap();
    assert_eq!(foo, "bar");

    let data = get_fixture_data!("abc" as SharedData).unwrap();
    assert_eq!(data.foo, "foo");
}

#[test]
fn it_works_2() {
    let result = add(2, 2);
    assert_eq!(result, 4);
}
