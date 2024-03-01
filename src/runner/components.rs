#[derive(Default)]
pub struct CmdHistory {
    history: Vec<String>,
    state: usize,
}

impl CmdHistory {
    pub fn push(&mut self, cmd: impl Into<String>) {
        self.history.push(cmd.into());
        self.state = self.history.len();
    }

    pub fn get_prev(&mut self) -> Option<String> {
        if self.state == 0 {
            return None;
        }
        self.state -= 1;
        self.history.get(self.state).cloned()
    }

    pub fn get_next(&mut self) -> Option<String> {
        if self.history.len() <= self.state {
            return None;
        }
        self.state += 1;
        let cmd = self.history.get(self.state)?;
        Some(cmd.to_owned())
    }
}
