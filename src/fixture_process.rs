use std::{
    pin::Pin,
    process::{Command, ExitStatus},
    task,
};

use anyhow::{anyhow, Context as _, Result};
use async_ctrlc::CtrlC;
use futures_util::{pin_mut, ready, Future};
use log::debug;
use pin_project_lite::pin_project;
use smol::process::{Child, Command as SmolCommand};

use crate::utils::CommandExt as _;

pin_project! {
    pub struct FixtureProcess {
        // #[pin]
        child: Child,
        #[pin]
        ctrlc: CtrlC,
    }
}

impl FixtureProcess {
    pub fn spawn(fixture_cmd: Command, ctrlc: CtrlC) -> Result<Self> {
        debug!("running {}", fixture_cmd.display());
        let mut child = SmolCommand::from(fixture_cmd)
            .spawn()
            .context("Error launching fixture")?;

        todo!()
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
