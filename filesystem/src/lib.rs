// @ref LLP 1030.002 D4 — platform operations keep the same owned-handle boundary.
#[cfg(unix)]
#[path = "directory.rs"]
mod backend;
#[cfg(windows)]
#[path = "directory_windows.rs"]
mod backend;
pub use backend::*;
use std::io;

pub struct Kind {
    pub directory: bool,
    pub regular: bool,
    pub len: u64,
    pub identity: (u64, u64),
}

impl Directory {
    /// Visit static files through owned parents, omitting Finder's `.DS_Store` metadata.
    /// Links and special files are refused.
    pub fn visit_files(
        &self,
        prefix: &str,
        visit: &mut impl FnMut(&str, &Directory, &str) -> io::Result<()>,
    ) -> io::Result<()> {
        for leaf in self.names()? {
            let path = format!("{prefix}{leaf}");
            match self.kind(&leaf)? {
                Kind {
                    directory: true, ..
                } => self
                    .child(&leaf, false)?
                    .visit_files(&format!("{path}/"), visit)?,
                Kind { regular: true, .. } if leaf == ".DS_Store" => {}
                Kind { regular: true, .. } => visit(&path, self, &leaf)?,
                _ => {
                    return Err(refuse(
                        "static app files must be regular files or directories",
                    ))
                }
            }
        }
        Ok(())
    }
}
