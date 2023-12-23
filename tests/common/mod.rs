use std::process::{Command, Output};

pub fn cargo_fixture(test: &str) -> Output {
    let exe = env!("CARGO_BIN_EXE_cargo-fixture");
    let fixture = format!("fixture_{test}");
    let callback = format!("{test}_callback");
    Command::new(exe)
        .args(["--fixture", &fixture])
        .args(["--", "--nocapture", "--exact", &callback])
        .output()
        .unwrap()
}

pub trait OutputExt {
    fn assert_success(&self);
    fn assert_error(&self, stderr_contains: &str);
}

impl OutputExt for Output {
    fn assert_success(&self) {
        let success = self.status.success();
        if !success {
            let stderr = String::from_utf8_lossy(&self.stderr).replace('\n', "\n  ");
            eprintln!("cargo fixture stderr:\n\n  {stderr}");
        }
        assert!(success);
    }

    fn assert_error(&self, stderr_contains: &str) {
        assert!(!self.status.success());
        let stderr = String::from_utf8_lossy(&self.stderr);
        assert!(stderr.contains(stderr_contains));
    }
}
