use crate::configs::GeneralAction;
use anyhow::Result;
use ratatui::backend::Backend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Stylize;
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::Frame;
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::process::{Child, Command};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

#[derive(Debug)]
pub struct EditorTerminal {
    pub active: bool,
    history: Vec<String>,
    process: Option<(Child, JoinHandle<()>)>,
    out_buffer: Arc<Mutex<Vec<String>>>,
    cmd_buffer: String,
}

impl EditorTerminal {
    pub fn render_with_remainder(&mut self, frame: &mut Frame<impl Backend>, screen: Rect) -> Rect {
        if !self.active {
            return screen;
        }
        self.poll_results();
        let screen_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Min(2)])
            .split(screen);
        let prompt = self.prompt();
        let mut list = self.get_list_widget();
        list.push(ListItem::new(Span::from(prompt).bold()));
        frame.render_widget(List::new(list).block(Block::default().borders(Borders::TOP)), screen_areas[1]);
        screen_areas[0]
    }

    fn get_list_widget(&mut self) -> Vec<ListItem> {
        self.history.iter().map(|line| ListItem::new(Span::from(line.to_owned()))).collect::<Vec<ListItem>>()
    }

    pub fn toggle(&mut self) {
        self.active = !self.active
    }

    pub fn new() -> Self {
        Self {
            active: false,
            history: Vec::new(),
            process: None,
            out_buffer: Arc::new(Mutex::new(Vec::new())),
            cmd_buffer: String::new(),
        }
    }

    pub async fn map(&mut self, general_action: &GeneralAction) -> bool {
        if !self.active {
            return false;
        }
        match general_action {
            GeneralAction::Char(ch) => self.cmd_buffer.push(*ch),
            GeneralAction::ToggleTerminal | GeneralAction::FileTreeModeOrCancelInput | GeneralAction::Exit => {
                self.active = false
            }
            GeneralAction::FinishOrSelect => {
                let _ = self.push_buffer().await;
            }
            _ => (),
        }
        true
    }

    fn poll_results(&mut self) {
        match self.out_buffer.lock() {
            Ok(mut guard) => self.history.extend(guard.drain(..)),
            Err(poisoned) => self.history.extend(poisoned.into_inner().drain(..)),
        }
    }

    fn prompt(&self) -> String {
        format!("> {}", self.cmd_buffer)
    }

    async fn push_buffer(&mut self) -> Result<()> {
        let mut inner = Command::new("sh")
            .arg("-c")
            .arg(self.cmd_buffer.as_str())
            .stderr(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        self.history.push(self.cmd_buffer.drain(..).collect());
        let out_handler = Arc::clone(&self.out_buffer);
        let stderr = FramedRead::new(inner.stderr.take().unwrap(), BytesCodec::new());
        let stdout = FramedRead::new(inner.stdout.take().unwrap(), BytesCodec::new());
        let mut stream = stdout.chain(stderr);
        let join_handler = tokio::spawn(async move {
            while let Some(Ok(bytes)) = stream.next().await {
                let out = String::from_utf8_lossy(&bytes);
                match out_handler.lock() {
                    Ok(mut guard) => guard.push(out.to_string()),
                    Err(poisoned) => poisoned.into_inner().push(out.to_string()),
                }
            }
        });
        self.process.replace((inner, join_handler));
        Ok(())
    }
}
