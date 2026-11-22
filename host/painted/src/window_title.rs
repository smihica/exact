//! The window title declared by the active `head` (LLP 1048.003 D1).

use exact_kernel::PropId;
use exact_runner::{DataSource, Runner};

/// The title last read by a native window, with the commit it came from.
#[derive(Default)]
pub struct WindowTitle {
    commit: Option<(u64, u64)>,
    authored: Option<String>,
}

impl WindowTitle {
    /// Read the active head after a commit. Return whether to publish the title.
    pub fn update(&mut self, runner: &Runner<impl DataSource>) -> bool {
        let commit = (runner.generation(), runner.kernel().epoch());
        if self.commit == Some(commit) {
            return false;
        }
        let title = if runner.kernel().has_prop(PropId::HeadTitle) {
            runner.head().title
        } else {
            None
        };
        let publish = self.commit.is_none() || title != self.authored;
        self.commit = Some(commit);
        self.authored = title;
        publish
    }

    /// The authored title, including an explicitly empty one.
    pub(crate) fn authored(&self) -> Option<&str> {
        self.authored.as_deref()
    }

    /// The caption for a window whose default title is the app name.
    pub fn caption<'a>(&'a self, app_name: &'a str) -> &'a str {
        self.authored().unwrap_or(app_name)
    }
}
