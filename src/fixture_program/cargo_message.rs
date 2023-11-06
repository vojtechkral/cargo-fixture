//! Subset of `cargo_metadata::Message` with only the bits that we need,
//! and async `parse_stream()`.

use std::{collections::HashSet, path::PathBuf};

use anyhow::{Context, Error, Result};
use futures_util::{Stream, TryStreamExt};
use log::trace;
use serde::Deserialize;
use smol::{
    future,
    io::{AsyncBufReadExt as _, AsyncRead, BufReader},
};

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "reason", rename_all = "kebab-case")]
pub enum Message {
    CompilerArtifact(Artifact),
    #[serde(other)]
    Other,
}

impl Message {
    pub fn parse_stream(
        input: impl AsyncRead + Unpin,
    ) -> impl Stream<Item = Result<Message>> + Unpin {
        BufReader::new(input)
            .lines()
            .map_err(Error::from)
            .and_then(move |line| {
                trace!("cargo build message: {line}");
                let res = serde_json::from_str(&line)
                    .with_context(|| format!("failed to deserialize cargo build message: {line}"));
                future::ready(res)
            })
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct Artifact {
    pub target: Target,
    pub executable: Option<PathBuf>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Target {
    pub name: String,
    pub kind: HashSet<String>,
}
