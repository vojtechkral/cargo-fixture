use std::{
    pin::Pin,
    task,
    time::{Duration, Instant},
};

use anyhow::{Context as _, Result};
use futures_util::{future::FusedFuture, Future, FutureExt as _, StreamExt};
use log::{debug, error, info, warn};
use smol::{
    process::{Child, Command as SmolCommand},
    Task, Timer,
};

use crate::{
    config::Config,
    utils::{CommandExt as _, ExitStatusExt},
};

type BoxFusedFuture<'a, T> = Pin<Box<dyn FusedFuture<Output = T> + Send + 'a>>;

pub struct FixtureProcess {
    child: Child,
    status_ft: BoxFusedFuture<'static, Result<()>>,
}

impl FixtureProcess {
    pub fn spawn_build(config: &Config) -> Result<Self> {
        info!("building fixture program...");
        Self::spawn(config, false)
    }

    pub fn spawn_run(config: &Config) -> Result<Self> {
        info!("setting up fixture...");
        Self::spawn(config, true)
    }

    fn spawn(config: &Config, run: bool) -> Result<Self> {
        let fixture_cmd = config.fixture_cmd(run);
        let verb = if run { "running" } else { "building" };

        debug!("running {}", fixture_cmd.display());
        let mut child = SmolCommand::from(fixture_cmd)
            .spawn()
            .with_context(|| format!("error {verb} fixture program"))?;

        let err_context = format!("{verb} fixture program failed");
        let status_ft = child
            .status()
            .map(move |res| {
                res.context(err_context.clone())
                    .and_then(|s| s.as_result(&err_context))
            })
            .fuse();
        let status_ft = Box::pin(status_ft) as BoxFusedFuture<'_, _>;

        Ok(Self { child, status_ft })
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
