use std::{
    fmt, fs,
    future::Future,
    path::Path,
    pin::Pin,
    process::Command,
    task::{ready, Context, Poll},
};

use log::trace;
use pin_project_lite::pin_project;

pub trait CommandExt {
    fn display<'a>(&'a self) -> CommandPrint<'a>;
}

impl CommandExt for Command {
    fn display<'a>(&'a self) -> CommandPrint<'a> {
        CommandPrint(self)
    }
}

pub struct CommandPrint<'a>(&'a Command);

impl<'a> fmt::Display for CommandPrint<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.get_program().to_string_lossy())?;
        for arg in self.0.get_args() {
            write!(f, " {}", arg.to_string_lossy())?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct RmGuard<P>(pub(crate) P)
where
    P: AsRef<Path>;

impl<P> Drop for RmGuard<P>
where
    P: AsRef<Path>,
{
    fn drop(&mut self) {
        let p = self.0.as_ref();
        trace!("removing {}", p.display());
        let _ = fs::remove_file(p);
    }
}

pin_project! {
    #[derive(Debug)]
    pub struct Map<Fut, F> {
        #[pin]
        future: Fut,
        mapper: Option<F>,
    }
}

impl<Fut, F, R> Map<Fut, F>
where
    Fut: Future,
    F: FnOnce(Fut::Output) -> R,
{
    pub fn new(future: Fut, mapper: F) -> Self {
        Self {
            future,
            mapper: Some(mapper),
        }
    }
}

impl<Fut, F, R> Future for Map<Fut, F>
where
    Fut: Future,
    F: FnOnce(Fut::Output) -> R,
{
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        this.mapper
            .as_ref()
            .expect("Map must not be polled after it returned `Poll::Ready`");

        let fut_res = ready!(this.future.poll(cx));
        let mapper = this.mapper.take().unwrap();
        Poll::Ready(mapper(fut_res))
    }
}

pub trait FutureExt: Future {
    fn map<R, F>(self, f: F) -> Map<Self, F>
    where
        F: FnOnce(Self::Output) -> R,
        Self: Sized;
}

impl<Fut> FutureExt for Fut
where
    Fut: Future,
{
    fn map<U, F>(self, mapper: F) -> Map<Self, F>
    where
        F: FnOnce(Self::Output) -> U,
        Self: Sized,
    {
        Map::new(self, mapper)
    }
}
