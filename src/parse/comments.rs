use std::mem::{replace, take};

#[derive(Debug)]
pub(super) struct Comments {
    state: State,
}

#[derive(Debug)]
enum State {
    Start,
    StartNewline,
    MaybeTrailing {
        trailing: String,
    },
    Detached {
        trailing: Option<String>,
        detached: Vec<String>,
    },
    Leading {
        trailing: Option<String>,
        detached: Vec<String>,
        leading: String,
    },
}

impl Comments {
    pub fn new() -> Comments {
        Comments {
            state: State::Start,
        }
    }

    pub fn comment(&mut self, comment: String) {
        self.state = match replace(&mut self.state, State::Start) {
            State::Start | State::StartNewline => State::MaybeTrailing { trailing: comment },
            State::MaybeTrailing { trailing } => State::Leading {
                trailing: None,
                detached: vec![trailing],
                leading: comment,
            },
            State::Detached { trailing, detached } => State::Leading {
                trailing,
                detached,
                leading: comment,
            },
            State::Leading {
                trailing,
                mut detached,
                leading,
            } => {
                detached.push(leading);
                State::Leading {
                    trailing,
                    detached,
                    leading: comment,
                }
            }
        }
    }

    pub fn newline(&mut self) {
        self.state = match replace(&mut self.state, State::Start) {
            State::Start | State::StartNewline => State::MaybeTrailing { trailing: comment },
            State::MaybeTrailing { trailing } => State::Leading {
                trailing: None,
                detached: vec![trailing],
                leading: comment,
            },
            State::Detached { trailing, detached } => State::Leading {
                trailing,
                detached,
                leading: comment,
            },
            State::Leading {
                trailing,
                mut detached,
                leading,
            } => {
                detached.push(leading);
                State::Leading {
                    trailing,
                    detached,
                    leading: comment,
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.state = State::Start;
    }

    pub fn take(&mut self) -> (Vec<String>, Option<String>) {
        match replace(&mut self.state, State::Start) {
            State::Start | State::StartNewline => (vec![], None),
            State::MaybeTrailing { trailing } => (vec![trailing], None),
            State::Detached { trailing, mut detached } => {
                if let Some(trailing) = trailing {
                    detached.insert(0, trailing)
                }
                (detached, None)
            },
            State::Leading { trailing, detached, leading } => {
                if let Some(trailing) = trailing {
                    detached.insert(0, trailing)
                }
                (detached, Some(leading))
            },
        }
    }

    pub fn take_trailing(&mut self) -> Option<String> {
        match replace(&mut self.state, State::Start) {
            State::Start | State::StartNewline => None,
            State::MaybeTrailing { trailing } => Some(trailing),
            State::Detached { trailing, detached } => {
                self.state = State::Detached { trailing: None, detached };
                return trailing;
            },
            State::Leading {
                trailing,
                mut detached,
                leading,
            } => {
                self.state = State::Leading { trailing: None, detached, leading };
                return trailing;
            }
        }
    }
}
