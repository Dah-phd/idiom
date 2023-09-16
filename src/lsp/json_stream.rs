use anyhow::{anyhow, Result};
use serde_json::{from_str, Value};
use tokio::process::{Child, ChildStderr, ChildStdout};
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

pub struct JsonRpc {
    inner: FramedRead<ChildStdout, BytesCodec>,
    inner_errors: FramedRead<ChildStderr, BytesCodec>,
    msg: String,
    parsed_objects: Vec<Value>,
    expected_len: usize,
}

impl JsonRpc {
    pub fn new(child: &mut Child) -> Result<Self> {
        let inner = child.stdout.take().ok_or(anyhow!("stdout"))?;
        let inner_errors = child.stderr.take().ok_or(anyhow!("stderr"))?;
        Ok(Self {
            inner: FramedRead::new(inner, BytesCodec::new()),
            inner_errors: FramedRead::new(inner_errors, BytesCodec::new()),
            msg: String::new(),
            parsed_objects: Vec::new(),
            expected_len: 0,
        })
    }

    pub async fn next(&mut self) -> Result<Value> {
        if !self.parsed_objects.is_empty() {
            return Ok(self.parsed_objects.remove(0)); // ensure all objects are sent
        }
        while self.parsed_objects.is_empty() {
            match self.inner.next().await.ok_or(anyhow!("Finished!"))? {
                Ok(result) => self.parse(result.into())?,
                Err(_) => return Err(anyhow!("Failed to read message!")),
            }
        }
        Ok(self.parsed_objects.remove(0))
    }

    fn parse(&mut self, msg: Vec<u8>) -> Result<()> {
        let new_message = String::from_utf8(msg)?;
        if self.msg.is_empty() {
            self.msg = new_message;
            self.update_expected_len()?;
        } else {
            self.msg.push_str(&new_message);
        }
        while let Some(object) = self.produce_object() {
            self.parsed_objects.push(object);
            self.update_expected_len()?;
        }
        Ok(())
    }

    pub fn produce_object(&mut self) -> Option<Value> {
        if self.msg.len() < self.expected_len {
            return None;
        }
        let object: Value = from_str(&self.msg[..self.expected_len]).ok()?;
        self.msg = self.msg.split_off(self.expected_len);
        Some(object)
    }

    pub fn update_expected_len(&mut self) -> Result<()> {
        if self.msg.starts_with("Content-Length:") && self.msg.contains("\r\n\r\n") {
            let msg_len: String = self.msg.chars().take_while(is_end_of_line).filter(|c| c.is_numeric()).collect();
            self.expected_len = msg_len.parse()?;
            self.msg = self.msg.drain(..).skip_while(|c| c != &'{').collect();
        } else if self.msg.is_empty() {
            self.expected_len = 0; // remove expectation if no message is left
        }
        if self.expected_len == 0 && !self.msg.starts_with('C') {
            self.msg.clear(); // clear unexpected string junk
        }
        Ok(())
    }
}

fn is_end_of_line(c: &char) -> bool {
    c != &'\r' && c != &'\n'
}
