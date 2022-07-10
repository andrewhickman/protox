use std::mem::take;

#[derive(Default, Debug, Clone)]
pub(super) struct Comments {
    detached: Vec<String>,
    current: Option<String>,
}

impl Comments {
    pub fn new() -> Comments {
        Comments::default()
    }

    pub fn comment(&mut self, comment: String) {
        self.newline();
        self.current = Some(comment);
    }

    pub fn newline(&mut self) {
        self.detached.extend(self.current.take());
    }

    pub fn reset(&mut self) {
        self.detached.clear();
        self.current = None;
    }

    #[allow(unused)]
    pub fn take(&mut self) -> (Vec<String>, Option<String>) {
        (take(&mut self.detached), take(&mut self.current))
    }
}
