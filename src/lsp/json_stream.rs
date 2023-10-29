use anyhow::{anyhow, Result};
use serde_json::{from_str, Value};
use std::sync::{Arc, Mutex};
use tokio::process::{Child, ChildStdout};
use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::utils::{into_guard, split_arc_mutex};

pub struct JsonRpc {
    inner: FramedRead<ChildStdout, BytesCodec>,
    #[allow(dead_code)]
    stderr_handler: JoinHandle<()>,
    errors: Arc<Mutex<Vec<String>>>,
    msg: String,
    parsed_que: Vec<Value>,
    expected_len: usize,
}

impl JsonRpc {
    pub fn new(child: &mut Child) -> Result<Self> {
        let inner = child.stdout.take().ok_or(anyhow!("stdout"))?;
        let mut stderr = FramedRead::new(child.stderr.take().ok_or(anyhow!("stderr"))?, BytesCodec::new());
        let (errors, errors_handler) = split_arc_mutex(Vec::new());
        Ok(Self {
            inner: FramedRead::new(inner, BytesCodec::new()),
            msg: String::new(),
            errors,
            parsed_que: Vec::new(),
            expected_len: 0,
            stderr_handler: tokio::task::spawn(async move {
                while let Some(Ok(err)) = stderr.next().await {
                    if let Ok(msg) = String::from_utf8(err.into()) {
                        into_guard(&errors_handler).push(msg);
                    }
                }
            }),
        })
    }

    pub async fn next(&mut self) -> Result<Value> {
        self.check_errors()?;
        if !self.parsed_que.is_empty() {
            return Ok(self.parsed_que.remove(0)); // ensure all objects are sent
        }
        while self.parsed_que.is_empty() {
            match self.inner.next().await.ok_or(anyhow!("Finished!"))? {
                Ok(msg) => self.parse(msg.into())?,
                Err(_) => return Err(anyhow!("Failed to read message!")),
            }
        }
        Ok(self.parsed_que.remove(0))
    }

    fn check_errors(&mut self) -> Result<()> {
        if let Ok(errors) = self.errors.try_lock() {
            if !errors.is_empty() {
                return Err(anyhow!(errors.join(" && ")));
            }
        }
        Ok(())
    }

    fn parse(&mut self, msg: Vec<u8>) -> Result<()> {
        self.push(msg)?;
        self.update_expected_len()?;
        while let Some(object) = self.parse_buffer() {
            self.parsed_que.push(object);
            self.update_expected_len()?;
        }
        Ok(())
    }

    fn push(&mut self, msg: Vec<u8>) -> Result<()> {
        let new_message = String::from_utf8(msg)?;
        if self.msg.is_empty() {
            self.msg = new_message;
        } else {
            self.msg.push_str(&new_message);
        };
        Ok(())
    }

    pub fn parse_buffer(&mut self) -> Option<Value> {
        if self.msg.is_empty() || self.msg.len() < self.expected_len {
            return None;
        }
        let object: Value = from_str(&self.msg[..self.expected_len]).ok()?;
        self.msg = self.msg.split_off(self.expected_len);
        self.expected_len = 0;
        Some(object)
    }

    pub fn update_expected_len(&mut self) -> Result<()> {
        if self.msg.starts_with("Content-Length:") && self.msg.contains("\r\n\r\n") {
            let msg_len: String = self.msg.chars().take_while(is_end_of_line).filter(|c| c.is_numeric()).collect();
            self.expected_len = msg_len.parse()?;
            self.msg = self.msg.drain(..).skip_while(|c| c != &'{').collect();
        }
        if self.expected_len == 0 && !self.msg.is_empty() && !self.msg.starts_with('C') {
            return Err(anyhow!("Failed to parse msg len!"));
        }
        Ok(())
    }
}

fn is_end_of_line(c: &char) -> bool {
    c != &'\r' && c != &'\n'
}
