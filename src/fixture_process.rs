use std::{pin::Pin, process::ExitStatus, task};

use anyhow::{anyhow, Context as _, Result};
use futures_util::{pin_mut, ready, Future};
use log::debug;
use pin_project_lite::pin_project;
use smol::process::{Child, Command as SmolCommand};

use crate::{
    config::Config,
    utils::{CommandExt as _, CtrlC},
};

pin_project! {
    pub struct FixtureProcess {
        child: Child,
        #[pin]
        ctrlc: CtrlC,
    }
}

impl FixtureProcess {
    pub fn spawn(config: &Config, ctrlc: CtrlC) -> Result<Self> {
        let fixture_cmd = config.fixture_cmd();
        debug!("running {}", fixture_cmd.display());
        let child = SmolCommand::from(fixture_cmd)
            .spawn()
            .context("Error launching fixture")?;

        Ok(Self { child, ctrlc })
    }
}

impl Future for FixtureProcess {
    type Output = Result<ExitStatus>;

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> task::Poll<Self::Output> {
        let this = self.project();

        ready!(this.ctrlc.poll(cx).map(|_| anyhow!("Interrupted")));

        let status = this.child.status();
        pin_mut!(status);
        status
            .poll(cx)
            .map(|res| res.context("fixture process failed"))
    }
}
