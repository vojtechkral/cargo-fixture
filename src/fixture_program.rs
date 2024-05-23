use std::{
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    task,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context as _, Result};
use futures_util::{future::FusedFuture, Future, FutureExt as _};
use log::{debug, error, info, warn};
use smol::{io, process::Child, stream::StreamExt, Task, Timer};

use crate::{
    config::Config,
    utils::{CommandExt, ExitStatusExt},
};

mod cargo_message;
use cargo_message::Message;

pub async fn build(config: &Config) -> Result<PathBuf> {
    info!("building fixture program...");
    let fixture_name = config.cli.fixture_name.clone();
    let cmd = config.fixture_build_cmd();
    debug!("running {}", cmd.display());

    let mut cmd = cmd.into_smol(Stdio::null(), Stdio::piped(), Stdio::inherit());
    let mut child = cmd.spawn()?;
    let mut stdout = child.stdout.take().unwrap();

    // Spawn a task that reads cargo's stdout and looks for the fixture artifact message:
    let artifact_find = smol::spawn(async move {
        let res = Message::parse_stream(&mut stdout)
            .find_map(|res| match res {
                Ok(Message::CompilerArtifact(artifact))
                    if artifact.target.name == fixture_name
                        && artifact.target.kind.contains("test")
                        && artifact.executable.is_some() =>
                {
                    Some(Ok(artifact.executable.unwrap()))
                }
                Err(err) => Some(Err(err)),
                _ => None,
            })
            .await;

        // Read and drop everything else from the pipe; we need to keep the pipe open
        // until the process exits so that it doesn't die on EPIPE.
        let _ = io::copy(stdout, io::sink()).await;

        res
    });

    // Wait for cargo to exit
    child.status().await?.as_result("cargo test")?;

    artifact_find
        .await
        .ok_or_else(|| anyhow!("fixture artifact not found in cargo JSON output"))?
        .context("error reading cargo JSON output")
}

pub fn run(config: &Config, fixture_bin: &Path) -> Result<FixtureProcess> {
    info!("setting up fixture...");
    let cmd = config.fixture_run_cmd(fixture_bin);
    debug!("running {}", cmd.display());

    let mut child = cmd
        .into_smol(Stdio::null(), Stdio::inherit(), Stdio::inherit())
        .spawn()
        .with_context(|| "error running fixture program".to_string())?;

    let err_context = "fixture program failed".to_string();
    let status_ft = child
        .status()
        .map(move |res| {
            res.context(err_context.clone())
                .and_then(|s| s.as_result(&err_context))
        })
        .fuse();

    Ok(FixtureProcess::new(child, status_ft))
}

type BoxFusedFuture<'a, T> = Pin<Box<dyn FusedFuture<Output = T> + Send + 'a>>;

pub struct FixtureProcess {
    child: Child,
    status_ft: BoxFusedFuture<'static, Result<()>>,
}

impl FixtureProcess {
    pub fn new(
        child: Child,
        status_ft: impl FusedFuture<Output = Result<()>> + Send + 'static,
    ) -> Self {
        Self {
            child,
            status_ft: Box::pin(status_ft),
        }
    }

    pub fn busy_logger(verb: &'static str) -> Task<()> {
        smol::spawn(async move {
            let start = Instant::now();
            let mut timer = Timer::interval(Duration::from_secs(10));
            while timer.next().await.is_some() {
                let delta = start.elapsed().as_secs();
                warn!("fixture process has still not {verb} after {delta}s (use Ctrl+C twice to kill the process)");
            }
        })
    }

    pub fn kill(&mut self) {
        warn!("Double Ctrl+C received, killing fixture process...");
        if let Err(err) = self.child.kill() {
            error!("Failed to kill fixture process: {err:?}");
        }
    }
}

impl Future for FixtureProcess {
    type Output = Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        self.status_ft.as_mut().poll(cx)
    }
}

impl FusedFuture for FixtureProcess {
    fn is_terminated(&self) -> bool {
        self.status_ft.is_terminated()
    }
}
