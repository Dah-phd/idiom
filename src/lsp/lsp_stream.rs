use serde_json::{from_str, Value};
use std::{
    fmt::Display,
    num::ParseIntError,
    sync::{Arc, Mutex, MutexGuard},
};
use thiserror::Error;
use tokio::{
    process::{Child, ChildStdout},
    task::JoinHandle,
};
use tokio_stream::StreamExt;
use tokio_util::codec::{BytesCodec, FramedRead};

/// Streams LSPMessage every time next is called - it handles receiving, deserialization and objec parsing.
/// LSPMessages are nothing more than wrapper around object determining type [Request, Notification, Response, Error, Unknown].
/// Fail conditions:
///  * stream end
///  * bad bytes received from Codec
///  * failure to parse message len
pub struct JsonRCP {
    inner: FramedRead<ChildStdout, BytesCodec>,
    _stderr: JoinHandle<()>,
    errors: Arc<Mutex<Vec<String>>>,
    str_buffer: String,
    buffer: Vec<u8>,
    parsed_que: Vec<Value>,
    expected_len: usize,
}

impl JsonRCP {
    pub fn new(child: &mut Child) -> Result<Self, RCPError> {
        let inner = child.stdout.take().ok_or(RCPError::StdoutTaken)?;
        let mut stderr = FramedRead::new(child.stderr.take().ok_or(RCPError::StderrTaken)?, BytesCodec::new());
        let errors = Arc::default();
        let errors_handler = Arc::clone(&errors);
        Ok(Self {
            inner: FramedRead::new(inner, BytesCodec::new()),
            str_buffer: String::new(),
            buffer: Vec::new(),
            errors,
            parsed_que: Vec::new(),
            expected_len: 0,
            _stderr: tokio::task::spawn(async move {
                while let Some(Ok(err)) = stderr.next().await {
                    if let Ok(msg) = String::from_utf8(err.into()) {
                        into_guard(&errors_handler).push(msg);
                    }
                }
            }),
        })
    }

    pub async fn next<T>(&mut self) -> Result<T, RCPError>
    where
        T: From<Value>,
        T: From<StdErrMessage>,
    {
        if let Some(err) = self.check_errors() {
            return Ok(err.into());
        };
        if !self.parsed_que.is_empty() {
            return Ok(self.parsed_que.remove(0).into()); // ensure all objects are sent
        }
        while self.parsed_que.is_empty() {
            let bytes = self.inner.next().await.ok_or(RCPError::StreamFinish)??;
            self.buffer.append(&mut bytes.to_vec());
            match std::str::from_utf8(&self.buffer) {
                Ok(msg) => {
                    self.str_buffer.push_str(msg);
                    self.buffer.clear();
                }
                Err(_) => continue, // buffer is not fully read
            };
            self.parse()?;
        }
        Ok(self.parsed_que.remove(0).into())
    }

    fn check_errors(&mut self) -> Option<StdErrMessage> {
        let mut errors = self.errors.try_lock().ok()?;
        errors.drain(..).reduce(to_lines).map(StdErrMessage)
    }

    fn parse(&mut self) -> Result<(), RCPError> {
        self.update_expected_len()?;
        while let Some(object) = self.parse_buffer() {
            self.parsed_que.push(object);
            self.update_expected_len()?;
        }
        Ok(())
    }

    pub fn parse_buffer(&mut self) -> Option<Value> {
        if self.str_buffer.is_empty() || self.str_buffer.len() < self.expected_len || self.expected_len == 0 {
            return None;
        }
        let object: Value = match from_str(&self.str_buffer[..self.expected_len]) {
            Ok(object) => object,
            Err(err) => {
                into_guard(&self.errors).push(err.to_string());
                self.hard_reset();
                return None;
            }
        };
        self.str_buffer = self.str_buffer.split_off(self.expected_len);
        self.expected_len = 0;
        Some(object)
    }

    fn hard_reset(&mut self) {
        if let Some(idx) = self.str_buffer.find("Content-Length") {
            self.str_buffer = self.str_buffer.split_off(idx);
            let _ = self.update_expected_len();
        } else {
            self.expected_len = 0;
            self.str_buffer.clear();
            self.buffer.clear();
        }
    }

    pub fn update_expected_len(&mut self) -> Result<(), RCPError> {
        if self.str_buffer.starts_with("Content-Length:") && self.str_buffer.contains("\r\n\r\n") {
            let msg_len: String =
                self.str_buffer.chars().take_while(is_end_of_line).filter(|c| c.is_numeric()).collect();
            self.expected_len = msg_len.parse()?;
            self.str_buffer = self.str_buffer.drain(..).skip_while(|c| c != &'{').collect();
        }
        if self.expected_len == 0 && !self.str_buffer.is_empty() && !self.str_buffer.starts_with('C') {
            return Err(RCPError::ParsingError);
        }
        Ok(())
    }
}

fn is_end_of_line(c: &char) -> bool {
    c != &'\r' && c != &'\n'
}

fn to_lines(mut a: String, b: String) -> String {
    a.push('\n');
    a.push_str(&b);
    a
}

fn into_guard<T>(mutex: &Mutex<T>) -> MutexGuard<T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

pub struct StdErrMessage(pub String);

#[derive(Error, Debug)]
pub enum RCPError {
    IOError(#[from] std::io::Error),
    ParsingHeaderError(#[from] ParseIntError),
    ParsingError,
    StderrTaken,
    StdoutTaken,
    StreamFinish,
}

impl Display for RCPError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IOError(err) => {
                f.write_str("RCP IO Error - unable to parse bytes: ")?;
                Display::fmt(err, f)
            }
            Self::ParsingError => f.write_str("RCP Error: Unable to parse message!"),
            Self::ParsingHeaderError(msg) => {
                f.write_str("RCP Error: Unable to parse message header!")?;
                Display::fmt(msg, f)
            }
            Self::StderrTaken => f.write_str("RCP Creation Error: Process stderr taken from process!"),
            Self::StdoutTaken => f.write_str("RCP Creation Error: Process stdout taken from process!"),
            Self::StreamFinish => f.write_str("RCP Stream Error: Steam finished!"),
        }
    }
}
