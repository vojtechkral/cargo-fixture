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
            .context("Error launching fixture")?;

        Ok(Self { child, ctrlc })
    }

    pub async fn join(mut self) -> Result<()> {
        let err_context = "fixture program failed";
        let status = self.child.status().map(|res| {
            res.context(err_context)
                .and_then(|s| s.as_result(err_context))
        });
        self.ctrlc.interruptible(status).await
    }
}
