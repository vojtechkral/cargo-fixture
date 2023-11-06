use std::{
    env, io,
    process::{self, Command, Stdio},
};

use cargo_fixture::TestClient;

pub mod common;
use common::ArgsReport;

#[smol_potat::main]
async fn main() {
    let args = env::args().collect::<Vec<_>>();

    // Since this program is used as a cargo replacement in the args test,
    // we need to forward metadata and fixture building commands to the real cargo:
    let cmd = args.get(1).map(|arg| arg.as_ref());
    if cmd == Some("metadata")
        || (cmd == Some("test") && args.iter().find(|&arg| arg == "fixture_args").is_some())
    {
        let cargo = env::var("CARGO_REAL").unwrap();
        let status = Command::new(cargo)
            .args(&args[1..])
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .unwrap()
            .code()
            .unwrap_or(0);

        process::exit(status);
    }

    let fixture_args = if env::var_os("CARGO_FIXTURE_SOCKET").is_some() {
        TestClient::connect(false)
            .await
            .unwrap()
            .get_value::<Vec<String>>("fixture-args")
            .await
            .unwrap()
    } else {
        vec![]
    };

    let report = ArgsReport {
        fixture_args,
        test_args: args,
    };
    serde_json::to_writer_pretty(io::stdout().lock(), &report).unwrap();
}
