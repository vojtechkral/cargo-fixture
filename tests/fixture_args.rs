use std::env;

use cargo_fixture::FixtureClient;

pub mod common;

#[smol_potat::main]
async fn main() {
    let mut fixture = FixtureClient::connect().await.unwrap();
    fixture.set_value("fixture-args", &[""; 0]).await.unwrap();

    let args = env::args().collect::<Vec<_>>();
    let cmd = args.get(1).expect("missing command").as_str();
    match cmd {
        "set_extra_cargo_test_args" => fixture.set_extra_cargo_test_args(&args[2..]).await,
        "set_extra_test_binary_args" => fixture.set_extra_test_binary_args(&args[2..]).await,
        "set_exec" => fixture.set_exec(&args[2..]).await,
        "report" => fixture.set_value("fixture-args", &args[2..]).await,
        _ => panic!("unknown command: {cmd}"),
    }
    .unwrap();

    fixture.ready().await.unwrap();
}
