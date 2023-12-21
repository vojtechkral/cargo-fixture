use anyhow::{Context as _, Result};
use futures_util::FutureExt as _;
use log::{debug, info};
use smol::process::{Child, Command as SmolCommand};

use crate::{
    config::Config,
    utils::{CommandExt as _, CtrlC, ExitStatusExt},
};

pub struct FixtureProcess {
    child: Child,
    ctrlc: CtrlC,
    verb: &'static str,
}

impl FixtureProcess {
    pub fn spawn_build(config: &Config, ctrlc: CtrlC) -> Result<Self> {
        info!("building fixture program...");
        Self::spawn(config, ctrlc, false)
    }

    pub fn spawn_run(config: &Config, ctrlc: CtrlC) -> Result<Self> {
        info!("setting up fixture...");
        Self::spawn(config, ctrlc, true)
    }

    // FIXME: err msgs when building

    fn spawn(config: &Config, ctrlc: CtrlC, run: bool) -> Result<Self> {
        let fixture_cmd = config.fixture_cmd(run);
        debug!("running {}", fixture_cmd.display());
        let child = SmolCommand::from(fixture_cmd)
            .spawn()
            .context("error launching fixture")?;
        let verb = if run { "running" } else { "building" };

        Ok(Self { child, ctrlc, verb })
    }

    pub async fn join(mut self) -> Result<()> {
        let err_context = || format!("{} fixture program failed", self.verb);
        let status = self.child.status().map(|res| {
            res.with_context(err_context)
                .and_then(|s| s.as_result(err_context))
        });
        self.ctrlc.interruptible(status).await
    }
}
