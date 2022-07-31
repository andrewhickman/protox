use std::{borrow::Cow, mem::take};

#[derive(Default, Clone)]
pub(super) struct Comments {
    leading_detached: Vec<String>,
    leading: Option<String>,
    trailing: Option<String>,

    is_trailing: bool,
    is_line_comment: bool,
}

impl Comments {
    pub fn new() -> Comments {
        Comments::default()
    }

    pub fn block_comment(&mut self, comment: Cow<str>) {
        if self.leading.is_some() {
            self.flush();
        }
        self.leading = Some(comment.into_owned());
    }

    pub fn line_comment(&mut self, comment: Cow<str>) {
        if self.leading.is_some() && !self.is_line_comment {
            self.flush();
        }
        match &mut self.leading {
            None => self.leading = Some(comment.into_owned()),
            Some(current) => current.push_str(comment.as_ref()),
        }
        self.is_line_comment = true;
    }

    pub fn detach(&mut self) {
        self.is_trailing = false;
    }

    pub fn flush(&mut self) {
        if self.is_trailing {
            debug_assert!(self.leading_detached.is_empty());
            self.trailing = self.leading.take();
            self.is_trailing = false;
        } else {
            self.leading_detached.extend(self.leading.take());
        }
        self.is_line_comment = false;
    }

    pub fn reset(&mut self) {
        self.leading = None;
        self.leading_detached.clear();
        self.trailing = None;

        self.is_trailing = true;
        self.is_line_comment = false;
    }

    pub fn take_leading(&mut self) -> (Vec<String>, Option<String>) {
        (take(&mut self.leading_detached), take(&mut self.leading))
    }

    pub fn take_trailing(&mut self) -> Option<String> {
        take(&mut self.trailing)
    }
}
