use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    net::IpAddr,
    path::{Path, PathBuf},
    process::{self, Command, Stdio},
    thread,
    time::Duration,
};

use anyhow::Context;
use serde::{Deserialize, Serialize};

pub fn cargo_fixture() -> CargoFixture {
    CargoFixture::new()
}

pub struct CargoFixture {
    cmd: Command,
    exact: bool,
    fixture_args: Vec<OsString>,
    exec: Vec<OsString>,
    print_args_exe: Option<String>,
    check_socket_exists: bool,
    exe_rm: RmGuard,
}

impl CargoFixture {
    fn new() -> Self {
        let (cmd, exe_rm) = if cfg!(windows) {
            // On Windows it's necessary to copy the cargo-fixture to a tmp location
            // because cargo gets invoked recursively / tests are run in parallel and attempt to overwrite it.

            // WARN: The exe's filename still needs to end in -fixture.exe
            // so that the cargo extension argument is parsed as expected.
            let exe = PathBuf::from(env!("CARGO_BIN_EXE_cargo-fixture"));
            let exe_name = format!("tmp-{}-cargo-fixture.exe", process::id());
            let exe_in_tmpdir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(&exe_name);
            fs::copy(exe, &exe_in_tmpdir).unwrap();
            let cmd = Command::new(&exe_in_tmpdir);
            (cmd, RmGuard::new(exe_in_tmpdir))
        } else {
            (
                Command::new(env!("CARGO_BIN_EXE_cargo-fixture")),
                RmGuard::phony(),
            )
        };

        Self {
            cmd,
            exact: true,
            fixture_args: vec![],
            exec: vec![],
            print_args_exe: None,

            // It's not a good idea to check the socket always,
            // because it's inherently racy, so we only do it in hang tests
            check_socket_exists: false,

            exe_rm,
        }
    }

    pub fn fixure_arg(mut self, arg: impl AsRef<OsStr>) -> Self {
        self.fixture_args.push("-A".into());
        self.fixture_args.push(arg.as_ref().to_owned());
        self
    }

    pub fn exec(mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Self {
        self.exec = args
            .into_iter()
            .map(|arg| arg.as_ref().to_owned())
            .collect();
        self
    }

    pub fn env(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        self.cmd.env(key, value);
        self
    }

    pub fn exact(mut self, exact: bool) -> Self {
        self.exact = exact;
        self
    }

    pub fn check_socket_exists(mut self, check_socket_exists: bool) -> Self {
        self.check_socket_exists = check_socket_exists;
        self
    }

    pub fn run_test(mut self, test_name: &'static str) -> Child {
        let fixture = format!("fixture_{test_name}");
        let callback = format!("{test_name}_callback");
        self.cmd.args(["-L", "debug", "--fixture", &fixture]);

        let confirm_file = if self.exec.is_empty() {
            self.cmd
                .args(&self.fixture_args)
                .args(["--", "--nocapture"]);

            if self.exact {
                self.cmd.arg("--exact");
            }
            self.cmd.arg(&callback);

            // Prepare the callback confirm file
            // - we need to know that cargo fixture actually called cargo such that
            self.cmd
                .env("CALLBACK_CONFIRM_ID", process::id().to_string());
            Some(RmGuard::new(Self::confirm_filename(test_name)))
        } else {
            self.cmd.arg("--exec").args(&self.exec);
            None
        };

        self.cmd
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        eprintln!("running cargo fixture: {:?}", self.cmd);

        let child = self.cmd.spawn().unwrap();
        Child::new(child, confirm_file, self.check_socket_exists, self.exe_rm)
    }

    #[track_caller]
    pub fn run_assert_args(
        mut self,
        args: &[impl AsRef<OsStr>],
        expected_fixture_args: &[&str],
        expected_test_args: &[&str],
    ) {
        let print_args_exe = self.print_args_exe();

        let report = self
            .cmd
            .arg("fixture") // verify this is ignored
            .args(["-L", "debug", "--fixture", "fixture_args"])
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .env("CARGO_REAL", env!("CARGO"))
            .env("CARGO", print_args_exe)
            .output()
            .unwrap()
            .parse_args_report();

        assert_eq!(
            report.fixture_args, expected_fixture_args,
            "fixture args not as expected"
        );
        assert_eq!(
            &report.test_args[1..],
            expected_test_args,
            "test args not as expected"
        );
    }

    pub fn run_assert_shell(mut self) {
        let print_args_exe = self.print_args_exe();

        let report = self
            .cmd
            .args([
                "-L",
                "debug",
                "--fixture",
                "fixture_args",
                "-A",
                "report",
                "--shell",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .env("SHELL", &print_args_exe)
            .output()
            .unwrap()
            .parse_args_report();

        assert!(report.fixture_args.is_empty(), "{report:?}");
        assert_eq!(report.test_args, &[print_args_exe.as_str()], "{report:?}");
    }

    pub fn print_args_exe(&mut self) -> String {
        self.print_args_exe
            .get_or_insert_with(|| {
                // Get the path to the binary, by running it via cargo with no args
                let cargo = env!("CARGO");
                Command::new(cargo)
                    .args(["test", "--test", "print_args"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit())
                    .output()
                    .unwrap()
                    .parse_args_report()
                    .test_args
                    .pop()
                    .unwrap()
            })
            .clone()
    }

    fn confirm_filename(test_name: &str) -> PathBuf {
        tmp_path(format!("{test_name}.callback-confirm"))
    }
}

pub struct Child {
    inner: process::Child,
    confirm_file: Option<RmGuard>,
    socket_path: PathBuf,
    _exe_rm: RmGuard,
}

impl Child {
    fn new(
        inner: process::Child,
        confirm_file: Option<RmGuard>,
        check_socket_exists: bool,
        exe_rm: RmGuard,
    ) -> Self {
        // Hack: we assume the tmpdir is a subdir of the target dir, which isn't guaranteed...
        let tmp_dir = &PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
        let target_dir = tmp_dir.parent().unwrap();
        let pid = inner.id();
        let socket_path = target_dir.join(format!(".cargo-fixture-{pid}.sock"));

        if check_socket_exists {
            socket_path.wait_until_exists();
        }

        Self {
            inner,
            confirm_file,
            socket_path,
            _exe_rm: exe_rm,
        }
    }

    pub fn output(self) -> Output {
        let output = self.inner.wait_with_output().unwrap();
        assert!(
            !self.socket_path.exists(),
            "cargo fixture didn't clean up socket file"
        );
        Output::new(output, self.confirm_file)
    }

    pub fn wait_fixture_hang(self, hang_file: &Path) -> Self {
        // In hang tests, fixture indicates to us that it's about to hang
        // with a special .hang file in CARGO_TARGET_TMPDIR
        hang_file.wait_until_exists();
        self
    }

    /// Send SIGINT repeatedly to kill stuck fixture.
    ///
    /// This is UNIX-only, as on Windows the Ctrl+C event can only be sent by process
    /// to its own groups of processes attached to the same console, which terminates
    /// the parent cargo test process as well.
    /// The break event can be sent to a specific process (if started with CREATE_NEW_PROCESS_GROUP),
    /// but it seems in that case only one event is needed to kill a hanging fixture,
    /// so it's probably not being handled by the ctrlc crate well? Not sure what's going on there
    /// but the test wasn't testing the double Ctrl+C feature.
    #[cfg(unix)]
    pub fn kill_fixture(self) -> Output {
        let pid = self.inner.id();

        let output = thread::scope(|scope| {
            scope.spawn(|| {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;

                let pid = Pid::from_raw(pid as _);
                while let Ok(_) = kill(pid, Some(Signal::SIGINT)) {
                    thread::sleep(Duration::from_millis(100));
                }
            });

            let output = self.inner.wait_with_output().unwrap();
            Output::new(output, self.confirm_file)
        });

        assert!(
            !self.socket_path.exists(),
            "cargo fixture didn't clean up socket file"
        );
        output
    }
}

pub struct Output {
    inner: process::Output,
    confirm_file: Option<RmGuard>,
}

impl Output {
    fn new(inner: process::Output, confirm_file: Option<RmGuard>) -> Self {
        Self {
            inner,
            confirm_file,
        }
    }

    #[track_caller]
    pub fn assert_success(&self) {
        let success = self.inner.status.success();
        if !success {
            let stderr = String::from_utf8_lossy(&self.inner.stderr).replace('\n', "\n  ");
            eprintln!("cargo fixture stderr:\n\n  {stderr}");
        }
        assert!(success);

        // Also check that callback has run:
        if let Some(confirm_file) = self.confirm_file.as_ref() {
            let err = format!(
                "It appears callback test didn't run (it didn't write CALLBACK_CONFIRM_ID to {})",
                confirm_file.path().display()
            );
            let id = fs::read_to_string(&confirm_file).expect(&err);
            assert_eq!(id, process::id().to_string(), "{err}",);
        }
    }

    #[track_caller]
    pub fn assert_error(&self, substr: &str) {
        assert!(!self.inner.status.success());
        let stderr = String::from_utf8_lossy(&self.inner.stderr);
        assert!(
            stderr.contains(substr),
            "cargo fixture stderr doesn't containt `{substr}`:\nstderr: {stderr}"
        );
    }
}

pub fn confirm_callback_ran(test_name: &str) {
    let confirm_file = CargoFixture::confirm_filename(test_name);
    let id = env::var("CALLBACK_CONFIRM_ID").unwrap();
    fs::write(&confirm_file, id.as_bytes()).unwrap();
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KvExample {
    pub foo: String,
    pub bar: IpAddr,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ArgsReport {
    pub fixture_args: Vec<String>,
    pub test_args: Vec<String>,
}

trait OutputExt {
    fn parse_args_report(self) -> ArgsReport;
}

impl OutputExt for process::Output {
    #[track_caller]
    fn parse_args_report(self) -> ArgsReport {
        assert!(self.status.success()); // not printing stderr here because it's inherited
        serde_json::from_slice::<ArgsReport>(&self.stdout)
            .with_context(|| {
                format!(
                    "could not parse ArgsReport, stdout: {}",
                    String::from_utf8_lossy(&self.stdout)
                )
            })
            .unwrap()
    }
}

pub fn hang_file(name: &str) -> RmGuard {
    let filename = format!("{}_{}.hang", name, process::id());
    RmGuard::new(tmp_path(&filename))
}

#[derive(Debug)]
pub struct RmGuard {
    path: Option<PathBuf>,
}

impl RmGuard {
    pub fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    pub fn phony() -> Self {
        Self { path: None }
    }

    pub fn path(&self) -> &Path {
        self.path.as_ref().unwrap().as_ref()
    }
}

impl AsRef<Path> for RmGuard {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}

impl Drop for RmGuard {
    fn drop(&mut self) {
        let _ = self.path.as_ref().map(fs::remove_file);
    }
}

pub fn tmp_path(filename: impl AsRef<str>) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(filename.as_ref())
}

pub fn fixture_hang() {
    // First write a file indicating to the test that the fixture is about to hang
    let filename = env::var_os("HANG_FILE").expect("HANG_FILE not set");
    fs::write(&filename, b"hang").unwrap();

    // Hang...
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

trait PathExt {
    fn wait_until_exists(&self);
}

impl<T> PathExt for T
where
    T: AsRef<Path>,
{
    fn wait_until_exists(&self) {
        while !self.as_ref().exists() {
            thread::sleep(Duration::from_millis(50));
        }
    }
}
