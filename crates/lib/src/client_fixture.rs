use serde::Serialize;

use crate::{
    rpc_socket::{ConnectionType, Request, RpcSocket},
    Error, Result,
};

/// An RPC client used from fixture code.
///
/// An instance is created using [`FixtureClient::connect()`].
pub struct FixtureClient {
    socket: RpcSocket,
}

impl FixtureClient {
    /// Connect to the parent `cargo fixture` process.
    pub async fn connect() -> Result<Self> {
        RpcSocket::connect(ConnectionType::Fixture)
            .await
            .map(|socket| Self { socket })
    }

    /// Request that an environment variable be set for `cargo test`.
    pub async fn set_env_var(
        &mut self,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<()> {
        let name = name.into();
        let value = value.into();

        if name.is_empty() || name.contains('=') || name.contains('\0') || value.contains('\0') {
            return Err(Error::InvalidSetEnv);
        }

        let req = Request::SetEnv { name, value };
        self.socket.call(req).await?.as_ok()
    }

    /// Request that multiple environment variables be set for `cargo test`.
    pub async fn set_env_vars(
        &mut self,
        vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Result<()> {
        // TODO: batch this when bumping RPC version
        for (name, value) in vars {
            self.set_env_var(name, value).await?;
        }

        Ok(())
    }

    /// Set additional CLI arguments to be passed to `cargo test`.
    ///
    /// No that these are arguments intended for the `cargo test` command itself, to pass arguments to the test binary,
    /// such as `--nocapture` or similar, use [`set_extra_test_binary_args()`][FixtureClient::set_extra_test_binary_args].
    pub async fn set_extra_cargo_test_args(
        &mut self,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<()> {
        let req = Request::SetExtraTestArgs {
            args: args.into_iter().map(Into::into).collect(),
        };
        self.socket.call(req).await?.as_ok()
    }

    /// Set additional CLI arguments to be passed to the test binary.
    ///
    /// When using CLI, these are usually passed via cargo using the `--` syntax, i.e. `cargo test -- args`...
    pub async fn set_extra_test_binary_args(
        &mut self,
        args: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<()> {
        let req = Request::SetExtraHarnessArgs {
            args: args.into_iter().map(Into::into).collect(),
        };
        self.socket.call(req).await?.as_ok()
    }

    /// Set a value in `cargo fixture`'s in-memory K-V storage.
    ///
    /// The value can be any serde-serializable value. After set, it can be retrieved by the test code.
    ///
    /// The K-V store internally uses JSON representation.
    pub async fn set_value(&mut self, key: impl Into<String>, value: impl Serialize) -> Result<()> {
        let value = serde_json::to_value(value)?;
        let req = Request::SetKeyValue {
            key: key.into(),
            value,
        };
        self.socket.call(req).await?.as_ok()
    }

    /// Replace the testing program to be executed to a custom one, along with arguments (if any).
    ///
    /// This will make `cargo fixture` run the provided program instead of the usual `cargo test` invocation.
    pub async fn set_exec(
        &mut self,
        exec: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<()> {
        let req = Request::SetExec {
            exec: exec.into_iter().map(Into::into).collect::<Vec<_>>(),
        };
        self.socket.call(req).await?.as_ok()
    }

    /// Signal to `cargo fixture` that the fixture is ready, starting the test run.
    ///
    /// This will by default run `cargo test` and return back a `bool` success status,
    /// once the test run is complete. Note that it may take an arbitrarily long time.
    pub async fn ready(&mut self) -> Result<bool> {
        self.socket.call(Request::Ready).await?.as_tests_finished()
    }
}
