pub mod common;
use common::cargo_fixture;

#[test]
fn args() {
    // standard run
    cargo_fixture().run_assert_args(
        &[
            "-A",
            "report",
            "-A",
            "fixture-arg1",
            "-A",
            "fixture-arg2",
            "--test",
            "some_test",
            "-j=1",
            "--some-other-flag",
            "--",
            "--nocapture",
        ],
        &["fixture-arg1", "fixture-arg2"],
        &[
            "test",
            "--features",
            "_fixture",
            "-j=1",
            "--test",
            "some_test",
            "--some-other-flag",
            "--",
            "--nocapture",
        ],
    );

    // using --exec
    let mut fixture = cargo_fixture();
    let print_args_exe = fixture.print_args_exe();
    fixture.run_assert_args(
        &[
            "-A",
            "report",
            "-A",
            "fixture-arg1",
            "-A",
            "fixture-arg2",
            "--exec",
            &print_args_exe,
            "foo",
            "bar",
            "--",
            "--nocapture",
        ],
        &["fixture-arg1", "fixture-arg2"],
        &["foo", "bar", "--", "--nocapture"],
    );

    // using set_extra_cargo_test_args()
    cargo_fixture().run_assert_args(
        &[
            "-A",
            "set_extra_cargo_test_args",
            "-A",
            "extra-arg1",
            "-A",
            "extra-arg2",
            "-j",
            "1",
            "--test",
            "some_test",
            "--some-other-flag",
            "--",
            "--nocapture",
        ],
        &[],
        &[
            "test",
            "--features",
            "_fixture",
            "-j",
            "1",
            "--test",
            "some_test",
            "--some-other-flag",
            "extra-arg1",
            "extra-arg2",
            "--",
            "--nocapture",
        ],
    );

    // using set_extra_test_binary_args()
    cargo_fixture().run_assert_args(
        &[
            "-A",
            "set_extra_test_binary_args",
            "-A",
            "extra-arg1",
            "-A",
            "extra-arg2",
            "--test",
            "some_test",
            "--some-other-flag",
            "--",
            "--nocapture",
        ],
        &[],
        &[
            "test",
            "--features",
            "_fixture",
            "--test",
            "some_test",
            "--some-other-flag",
            "--",
            "--nocapture",
            "extra-arg1",
            "extra-arg2",
        ],
    );

    // using set_exec()
    cargo_fixture().run_assert_args(
        &[
            "-A",
            "set_exec",
            "-A",
            &print_args_exe,
            "-A",
            "exec-arg1",
            "-A",
            "exec-arg2",
            "--test",
            "some_test",
            "--some-other-flag",
            "--",
            "--nocapture",
        ],
        &[],
        &["exec-arg1", "exec-arg2"],
    );

    // --exec overrides set_exec()
    cargo_fixture().run_assert_args(
        &[
            "-A",
            "set_exec",
            "-A",
            &print_args_exe,
            "-A",
            "exec-arg1",
            "-A",
            "exec-arg2",
            "--exec",
            &print_args_exe,
            "exec-cli-arg1",
            "exec-cli-arg2",
        ],
        &[],
        &["exec-cli-arg1", "exec-cli-arg2"],
    );
}
