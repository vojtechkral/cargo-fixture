use std::{fs, path::Path};

use log::trace;

#[derive(Debug)]
pub(crate) struct RmGuard<'a>(pub(crate) &'a Path);

impl<'a> Drop for RmGuard<'a> {
    fn drop(&mut self) {
        trace!("removing {}", self.0.display());
        let _ = fs::remove_file(self.0);
    }
}
