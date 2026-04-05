use serde_json::{Value, from_str};
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

const RPC_HEADER: &str = "Content-Length:";
const BODY_SEP: &str = "\r\n\r\n";

/// Streams LSPMessage every time next is called - it handles receiving, deserialization and objec parsing.
/// LSPMessages are nothing more than wrapper around object determining type [Request, Notification, Response, Error, Unknown].
/// Fail conditions:
///  * stream end
///  * bad bytes received from Codec
///  * failure to parse message len
pub struct JsonRPC {
    inner: FramedRead<ChildStdout, BytesCodec>,
    stderr: JoinHandle<()>,
    errors: Arc<Mutex<Vec<String>>>,
    str_buffer: String,
    buffer: Vec<u8>,
    parsed_que: Vec<Value>,
    expected_len: usize,
}

impl JsonRPC {
    pub fn tokio_rt_new(child: &mut Child) -> Result<Self, RCPError> {
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
            stderr: tokio::spawn(async move {
                let mut buffer = Vec::new();
                while let Some(Ok(err)) = stderr.next().await {
                    buffer.extend(err.into_iter());
                    if let Ok(msg) = std::str::from_utf8(&buffer) {
                        into_guard(&errors_handler).push(msg.to_owned());
                        buffer.clear();
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
            self.buffer.extend(bytes.into_iter());

            if let Ok(msg) = std::str::from_utf8(&self.buffer) {
                self.str_buffer.push_str(msg);
                self.buffer.clear();
                self.parse()?;
            };
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

    fn parse_buffer(&mut self) -> Option<Value> {
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
        self.str_buffer.drain(..self.expected_len);
        self.expected_len = 0;
        Some(object)
    }

    fn hard_reset(&mut self) {
        if let Some(idx) = self.str_buffer.find(RPC_HEADER) {
            self.str_buffer.drain(..idx);
            let _ = self.update_expected_len();
        } else {
            self.expected_len = 0;
            self.str_buffer.clear();
            self.buffer.clear();
        }
    }

    fn update_expected_len(&mut self) -> Result<(), RCPError> {
        // split header with msg len
        let header = self.str_buffer.strip_prefix(RPC_HEADER).and_then(|cleaned| cleaned.split_once(BODY_SEP));
        if let Some((msg_len, _body)) = header {
            self.expected_len = msg_len.trim().parse()?;
            let header_end = RPC_HEADER.len() + BODY_SEP.len() + msg_len.len();
            self.str_buffer.drain(..header_end);
        }
        if self.expected_len == 0 && !self.str_buffer.is_empty() && !self.str_buffer.starts_with('C') {
            return Err(RCPError::ParsingError);
        }
        Ok(())
    }
}

fn to_lines(mut a: String, b: String) -> String {
    a.push('\n');
    a.push_str(&b);
    a
}

fn into_guard<'a, T>(mutex: &'a Mutex<T>) -> MutexGuard<'a, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl Drop for JsonRPC {
    fn drop(&mut self) {
        self.stderr.abort();
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

#[cfg(test)]
mod tests {
    use super::{Arc, BytesCodec, FramedRead, JsonRPC};
    use crate::utils::SHELL;
    use serde_json::json;
    use std::process::Stdio;
    use tokio::process::Command;

    #[tokio::test]
    async fn test_json_rpc_2_message_parsing() {
        let mut fake_process = Command::new(SHELL).kill_on_drop(true).stdout(Stdio::piped()).spawn().unwrap();
        let stdout = fake_process.stdout.take().unwrap();

        let mut fake_server = JsonRPC {
            inner: FramedRead::new(stdout, BytesCodec::new()),
            stderr: tokio::spawn(async {}),
            errors: Arc::default(),
            str_buffer: String::default(),
            buffer: Vec::default(),
            parsed_que: Vec::default(),
            expected_len: 0,
        };

        let example_msg = "{\
            \"jsonrpc\":\"2.0\",\
            \"id\":1,\
            \"method\": \"textDocument/completion\",\
            \"params\":{\"param\":1, \"data\":\"text\", \"l\": [\"a\",\"b\",3]}}";

        fake_server.str_buffer = format!("Content-Length:{}\r\n\r\n{}", example_msg.len(), example_msg);
        fake_server.parse().unwrap();
        let expected = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "textDocument/completion",
            "params": {"param": 1, "data": "text", "l":["a", "b", 3]}
        });

        assert_eq!(fake_server.parsed_que.pop(), Some(expected.clone()));
        assert_eq!(fake_server.parsed_que.pop(), None);
        assert_eq!("", fake_server.str_buffer);

        // incomplete second msg
        fake_server.str_buffer =
            format!("Content-Length:{}\r\n\r\n{}Content-Length:3\r\n\r", example_msg.len(), example_msg);
        fake_server.parse().unwrap();
        assert_eq!(fake_server.parsed_que.pop(), Some(expected.clone()));
        assert_eq!(fake_server.parsed_que.pop(), None);
        assert_eq!("Content-Length:3\r\n\r", fake_server.str_buffer);
        assert_eq!(0, fake_server.expected_len);

        // incomplete second msg with header send
        fake_server.str_buffer =
            format!("Content-Length:{}\r\n\r\n{}Content-Length:3\r\n\r\n", example_msg.len(), example_msg);
        fake_server.parse().unwrap();
        assert_eq!(fake_server.parsed_que.pop(), Some(expected.clone()));
        assert_eq!(fake_server.parsed_que.pop(), None);
        assert_eq!("", fake_server.str_buffer);
        assert_eq!(3, fake_server.expected_len);

        // incomplte message started
        fake_server.str_buffer =
            format!("Content-Length:{}\r\n\r\n{}Content-Length:10\r\n\r\n{{\"as", example_msg.len(), example_msg);
        fake_server.parse().unwrap();
        assert_eq!(fake_server.parsed_que.pop(), Some(expected.clone()));
        assert_eq!(fake_server.parsed_que.pop(), None);
        assert_eq!("{\"as", fake_server.str_buffer);
        assert_eq!(10, fake_server.expected_len);

        // two messages and a part
        fake_server.str_buffer = format!(
            "Content-Length:{}\r\n\r\n{}Content-Length:{}\r\n\r\n{}Content-Length:10\r\n\r\n{{\"as",
            example_msg.len(),
            example_msg,
            example_msg.len(),
            example_msg,
        );
        fake_server.parse().unwrap();
        assert_eq!(fake_server.parsed_que.pop(), Some(expected.clone()));
        assert_eq!(fake_server.parsed_que.pop(), Some(expected.clone()));
        assert_eq!(fake_server.parsed_que.pop(), None);
        assert_eq!("{\"as", fake_server.str_buffer);
        assert_eq!(10, fake_server.expected_len);
    }
}
