use super::{
    create_semantic_capabilities,
    tokens::generic::GenericToken,
    tokens::lobster::Pincer,
    tokens::placeholder::PlaceholderToken,
    tokens::python::PyToken,
    tokens::rust::Rustacean,
    tokens::ts::TSToken,
    tokens::PositionedTokenParser,
    utils::{full_tokens, partial_tokens, swap_content, utf16_encoder, utf32_encoder, utf8_encoder},
};
use crate::{
    configs::FileType,
    lsp::{
        local::{Definitions, LangStream, PositionedToken, Responses},
        payload::Payload,
        LSPError, LSPResult, Response,
    },
    workspace::CursorPosition,
};
use lsp_types::{
    notification::{DidOpenTextDocument, Notification},
    CompletionOptions, DidOpenTextDocumentParams, PositionEncodingKind, SemanticTokens, SemanticTokensRangeResult,
    SemanticTokensResult, ServerCapabilities, TextDocumentContentChangeEvent, TextDocumentItem,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, Uri,
};
use serde_json::{from_str, from_value, to_value, Value};
use std::{collections::HashMap, sync::Arc};
use tokio::{io::AsyncWriteExt, process::ChildStdin, sync::mpsc::UnboundedReceiver, task::JoinHandle};

type UtfEncoder = fn(lsp_types::Position, &[String]) -> CursorPosition;

enum EnrichType {
    Sync,
    Tokens,
    TokensSync,
    TokensAutocomplete,
    TokensSyncAutocomplete,
}

impl EnrichType {
    fn determine_and_updated(capabilities: &mut ServerCapabilities) -> Option<Self> {
        let has_tokens = capabilities.semantic_tokens_provider.is_some();
        let has_autocomplete = capabilities.completion_provider.is_some();
        let has_increment_sync = matches!(
            capabilities.text_document_sync,
            Some(
                TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL)
                    | TextDocumentSyncCapability::Options(TextDocumentSyncOptions {
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        ..
                    })
            )
        );
        match (has_tokens, has_autocomplete, has_increment_sync) {
            (false, false, false) => {
                capabilities.semantic_tokens_provider.replace(create_semantic_capabilities());
                capabilities
                    .text_document_sync
                    .replace(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL));
                capabilities.completion_provider.replace(CompletionOptions::default());
                Some(Self::TokensSyncAutocomplete)
            }
            (false, false, true) => {
                capabilities.semantic_tokens_provider.replace(create_semantic_capabilities());
                capabilities.completion_provider.replace(CompletionOptions::default());
                Some(Self::TokensAutocomplete)
            }
            (false, true, false) => {
                capabilities
                    .text_document_sync
                    .replace(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL));
                capabilities.semantic_tokens_provider.replace(create_semantic_capabilities());
                Some(Self::TokensSync)
            }
            (false, true, true) => {
                capabilities.semantic_tokens_provider.replace(create_semantic_capabilities());
                Some(Self::Tokens)
            }
            (true, true, false) => {
                capabilities
                    .text_document_sync
                    .replace(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::INCREMENTAL));
                Some(Self::Sync)
            }
            _ => None,
        }
    }
}

pub fn build_with_enrichment(
    mut rx: UnboundedReceiver<Payload>,
    mut lsp_stdin: ChildStdin,
    file_type: FileType,
    responses: Arc<Responses>,
    capabilities: &mut ServerCapabilities,
) -> JoinHandle<LSPResult<()>> {
    let enrich_type = match EnrichType::determine_and_updated(capabilities) {
        Some(enrichment) => enrichment,
        None => {
            return tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    if let Ok(lsp_msg_text) = msg.try_stringify() {
                        lsp_stdin.write_all(lsp_msg_text.as_bytes()).await?;
                        lsp_stdin.flush().await?;
                    }
                }
                Ok(())
            })
        }
    };
    let encoding = capabilities.position_encoding.to_owned();

    match (file_type, enrich_type) {
        (.., EnrichType::Sync) => {
            tokio::spawn(EnrichedLSP::<PlaceholderToken>::sync_parser(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Python, EnrichType::Tokens) => {
            tokio::spawn(EnrichedLSP::<PyToken>::run(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Python, EnrichType::TokensSync) => {
            tokio::spawn(EnrichedLSP::<PyToken>::run_with_sync(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Python, EnrichType::TokensAutocomplete) => {
            tokio::spawn(EnrichedLSP::<PyToken>::run_with_completion(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Python, EnrichType::TokensSyncAutocomplete) => {
            tokio::spawn(EnrichedLSP::<PyToken>::run_with_sync_and_completion(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Lobster, EnrichType::Tokens) => {
            tokio::spawn(EnrichedLSP::<Pincer>::run(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Lobster, EnrichType::TokensSync) => {
            tokio::spawn(EnrichedLSP::<Pincer>::run_with_sync(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Lobster, EnrichType::TokensAutocomplete) => {
            tokio::spawn(EnrichedLSP::<Pincer>::run_with_completion(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Lobster, EnrichType::TokensSyncAutocomplete) => {
            tokio::spawn(EnrichedLSP::<Pincer>::run_with_sync_and_completion(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Rust, EnrichType::Tokens) => {
            tokio::spawn(EnrichedLSP::<Rustacean>::run(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Rust, EnrichType::TokensSync) => {
            tokio::spawn(EnrichedLSP::<Rustacean>::run_with_sync(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Rust, EnrichType::TokensAutocomplete) => {
            tokio::spawn(EnrichedLSP::<Rustacean>::run_with_completion(rx, lsp_stdin, responses, encoding))
        }
        (FileType::Rust, EnrichType::TokensSyncAutocomplete) => {
            tokio::spawn(EnrichedLSP::<Rustacean>::run_with_sync_and_completion(rx, lsp_stdin, responses, encoding))
        }
        (FileType::JavaScript | FileType::TypeScript, EnrichType::Tokens) => {
            tokio::spawn(EnrichedLSP::<TSToken>::run(rx, lsp_stdin, responses, encoding))
        }
        (FileType::JavaScript | FileType::TypeScript, EnrichType::TokensSync) => {
            tokio::spawn(EnrichedLSP::<TSToken>::run_with_sync(rx, lsp_stdin, responses, encoding))
        }
        (FileType::JavaScript | FileType::TypeScript, EnrichType::TokensAutocomplete) => {
            tokio::spawn(EnrichedLSP::<TSToken>::run_with_completion(rx, lsp_stdin, responses, encoding))
        }
        (FileType::JavaScript | FileType::TypeScript, EnrichType::TokensSyncAutocomplete) => {
            tokio::spawn(EnrichedLSP::<TSToken>::run_with_sync_and_completion(rx, lsp_stdin, responses, encoding))
        }
        (.., EnrichType::Tokens) => tokio::spawn(EnrichedLSP::<GenericToken>::run(rx, lsp_stdin, responses, encoding)),
        (.., EnrichType::TokensSync) => {
            tokio::spawn(EnrichedLSP::<GenericToken>::run_with_sync(rx, lsp_stdin, responses, encoding))
        }
        (.., EnrichType::TokensAutocomplete) => {
            tokio::spawn(EnrichedLSP::<GenericToken>::run_with_completion(rx, lsp_stdin, responses, encoding))
        }
        (.., EnrichType::TokensSyncAutocomplete) => {
            tokio::spawn(EnrichedLSP::<GenericToken>::run_with_sync_and_completion(rx, lsp_stdin, responses, encoding))
        }
    }
}

/// Enrichment allows to imporve LSP capabilities by running process that will handle smaller process before the process
struct EnrichedLSP<T: LangStream> {
    documents: HashMap<Uri, DocumentData<T>>,
    definitions: Definitions,
    responses: Arc<Responses>,
    parser: PositionedTokenParser<T>,
    utf_position: UtfEncoder,
}

impl<T: LangStream> EnrichedLSP<T> {
    fn from_encoding(responses: Arc<Responses>, encoding: Option<PositionEncodingKind>) -> Self {
        match encoding.as_ref().map(|e| e.as_str()) {
            Some("utf-8") => Self {
                documents: HashMap::new(),
                definitions: T::init_definitions(),
                responses,
                parser: PositionedToken::<T>::utf8,
                utf_position: utf8_encoder,
            },
            Some("utf-32") => Self {
                documents: HashMap::new(),
                definitions: T::init_definitions(),
                responses,
                parser: PositionedToken::<T>::utf32,
                utf_position: utf32_encoder,
            },
            _ => Self {
                documents: HashMap::new(),
                definitions: T::init_definitions(),
                responses,
                parser: PositionedToken::<T>::utf16,
                utf_position: utf16_encoder,
            },
        }
    }

    async fn sync_parser(
        mut rx: UnboundedReceiver<Payload>,
        mut lsp_stdin: ChildStdin,
        responses: Arc<Responses>,
        encoding: Option<PositionEncodingKind>,
    ) -> LSPResult<()> {
        let mut local_lsp = Self::from_encoding(responses, encoding);
        while let Some(payload) = rx.recv().await {
            if let Payload::Sync(uri, version, change_events) = payload {
                let doc = local_lsp
                    .documents
                    .get_mut(&uri)
                    .ok_or_else(|| LSPError::internal("Unable to find document during sync coersion!"))?;
                let full_text = doc.sync_to_full_sync(&change_events, local_lsp.parser);
                let msg = Payload::FullSync(uri, version, full_text).try_stringify()?;
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            } else if let Ok(msg) = payload.try_stringify() {
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            }
        }
        Ok(())
    }

    async fn run(
        mut rx: UnboundedReceiver<Payload>,
        mut lsp_stdin: ChildStdin,
        responses: Arc<Responses>,
        encoding: Option<PositionEncodingKind>,
    ) -> LSPResult<()> {
        let mut local_lsp = Self::from_encoding(responses, encoding);
        while let Some(payload) = rx.recv().await {
            if let Some(msg) = local_lsp.pre_process(payload)? {
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            };
        }
        Ok(())
    }

    async fn run_with_completion(
        mut rx: UnboundedReceiver<Payload>,
        mut lsp_stdin: ChildStdin,
        responses: Arc<Responses>,
        encoding: Option<PositionEncodingKind>,
    ) -> LSPResult<()> {
        let mut local_lsp = Self::from_encoding(responses, encoding);
        local_lsp.definitions = T::init_definitions();
        while let Some(payload) = rx.recv().await {
            if let Payload::Completion(uri, cursor, id) = payload {
                local_lsp.autocomplete(uri, cursor, id);
            } else if let Some(msg) = local_lsp.pre_process(payload)? {
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            }
        }
        Ok(())
    }

    async fn run_with_sync(
        mut rx: UnboundedReceiver<Payload>,
        mut lsp_stdin: ChildStdin,
        responses: Arc<Responses>,
        encoding: Option<PositionEncodingKind>,
    ) -> LSPResult<()> {
        let mut local_lsp = Self::from_encoding(responses, encoding);
        local_lsp.definitions = T::init_definitions();
        while let Some(payload) = rx.recv().await {
            if let Payload::Sync(uri, version, change_events) = payload {
                let doc = local_lsp
                    .documents
                    .get_mut(&uri)
                    .ok_or_else(|| LSPError::internal("Unable to find document during sync coersion!"))?;
                let full_text = doc.sync_to_full_sync(&change_events, local_lsp.parser);
                let msg = Payload::FullSync(uri, version, full_text).try_stringify()?;
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            } else if let Some(msg) = local_lsp.pre_process(payload)? {
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            }
        }
        Ok(())
    }

    async fn run_with_sync_and_completion(
        mut rx: UnboundedReceiver<Payload>,
        mut lsp_stdin: ChildStdin,
        responses: Arc<Responses>,
        encoding: Option<PositionEncodingKind>,
    ) -> LSPResult<()> {
        let mut local_lsp = Self::from_encoding(responses, encoding);
        local_lsp.definitions = T::init_definitions();
        while let Some(payload) = rx.recv().await {
            if let Payload::Completion(uri, cursor, id) = payload {
                local_lsp.autocomplete(uri, cursor, id);
            } else if let Payload::Sync(uri, version, change_events) = payload {
                let doc = local_lsp
                    .documents
                    .get_mut(&uri)
                    .ok_or_else(|| LSPError::internal("Unable to find document during sync coersion!"))?;
                let full_text = doc.sync_to_full_sync(&change_events, local_lsp.parser);
                let msg = Payload::FullSync(uri, version, full_text).try_stringify()?;
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            } else if let Some(msg) = local_lsp.pre_process(payload)? {
                lsp_stdin.write_all(msg.as_bytes()).await?;
                lsp_stdin.flush().await?;
            }
        }
        Ok(())
    }

    fn pre_process(&mut self, payload: Payload) -> LSPResult<Option<String>> {
        match payload {
            Payload::Direct(data) => {
                self.direct_parsing(&data)?;
                Ok(Some(data))
            }
            Payload::Tokens(uri, id) => {
                let data = match self.documents.get(&uri) {
                    Some(doc) => full_tokens(&doc.tokens),
                    None => vec![],
                };
                let tokens = SemanticTokensResult::Tokens(SemanticTokens { result_id: None, data });
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(error) => Response { id, result: None, error: Some(Value::String(error.to_string())) },
                };
                self.responses.lock().unwrap().insert(id, response);
                Ok(None)
            }
            Payload::PartialTokens(uri, range, id, ..) => {
                let data = match self.documents.get(&uri) {
                    Some(doc) => {
                        let start = CursorPosition::from(range.start);
                        let end = CursorPosition::from(range.end);
                        partial_tokens(&doc.tokens, start, end)
                    }
                    None => vec![],
                };
                let tokens = SemanticTokensRangeResult::Tokens(SemanticTokens { result_id: None, data });
                let response = match to_value(tokens) {
                    Ok(value) => Response { id, result: Some(value), error: None },
                    Err(err) => Response { id, result: None, error: Some(Value::String(err.to_string())) },
                };
                self.responses.lock().unwrap().insert(id, response);
                Ok(None)
            }
            Payload::Sync(uri, version, change_events) => {
                if let Some(doc) = self.documents.get_mut(&uri) {
                    doc.sync(&change_events, self.parser);
                };
                Ok(Payload::Sync(uri, version, change_events).try_stringify().ok())
            }
            Payload::FullSync(uri, version, full_text) => {
                if let Some(doc) = self.documents.get_mut(&uri) {
                    doc.full_sync(&full_text, self.parser);
                };
                Ok(Payload::FullSync(uri, version, full_text).try_stringify().ok())
            }
            _ => Ok(payload.try_stringify().ok()),
        }
    }

    fn autocomplete(&mut self, uri: Uri, _cursor: CursorPosition, id: i64) {
        let completion_response = match self.documents.get(&uri) {
            Some(doc) => self.definitions.to_completions(&doc.tokens),
            None => vec![],
        };
        let response = match to_value(completion_response) {
            Ok(value) => Response { id, result: Some(value), error: None },
            Err(err) => Response { id, result: None, error: Some(Value::String(err.to_string())) },
        };
        self.responses.lock().unwrap().insert(id, response);
    }

    fn direct_parsing(&mut self, data: &str) -> Result<(), LSPError> {
        let (_h, msg) = data.split_once("\r\n\r\n").ok_or_else(|| LSPError::internal("Message header not found!"))?;
        let val = from_str::<Value>(msg)?;
        match val
            .as_object()
            .ok_or_else(|| LSPError::internal("Unexpected message format!"))?
            .get("method")
            .and_then(|meth| meth.as_str())
            .ok_or_else(|| LSPError::internal("No method found ot message!"))?
        {
            DidOpenTextDocument::METHOD => self
                .file_did_open(val)
                .ok_or_else(|| LSPError::internal("Failed to parse didOpenDocument notification!")),
            _ => Ok(()),
        }
    }

    fn file_did_open(&mut self, val: Value) -> Option<()> {
        let DidOpenTextDocumentParams { text_document: TextDocumentItem { uri, text, .. } } =
            from_value::<DidOpenTextDocumentParams>(val.get("params").cloned()?).ok()?;
        self.documents.insert(uri, DocumentData::open(text, self.parser, self.utf_position));
        Some(())
    }
}

pub struct DocumentData<T: LangStream> {
    text: Vec<String>,
    tokens: Vec<Vec<PositionedToken<T>>>,
    utf_position: UtfEncoder,
}

impl<T: LangStream> DocumentData<T> {
    fn open(text: String, parser: PositionedTokenParser<T>, utf_position: UtfEncoder) -> Self {
        let text = text.split('\n').map(ToOwned::to_owned).collect();
        let mut doc = Self { text, tokens: vec![], utf_position };
        T::parse(doc.text.iter().map(|t| t.as_str()), &mut doc.tokens, parser);
        doc
    }

    fn sync(&mut self, change_events: &[TextDocumentContentChangeEvent], parser: PositionedTokenParser<T>) {
        for change in change_events {
            let range = change.range.unwrap();
            let from = (self.utf_position)(range.start, &self.text);
            let to = (self.utf_position)(range.end, &self.text);
            swap_content(&mut self.text, &change.text, from, to);
        }
        T::parse(self.text.iter().map(|t| t.as_str()), &mut self.tokens, parser);
    }

    fn sync_to_full_sync(
        &mut self,
        change_events: &[TextDocumentContentChangeEvent],
        parser: PositionedTokenParser<T>,
    ) -> String {
        self.sync(change_events, parser);
        self.stringify()
    }

    fn full_sync(&mut self, new_text: &str, parser: PositionedTokenParser<T>) {
        self.text = new_text.split('\n').map(ToOwned::to_owned).collect();
        T::parse(self.text.iter().map(|t| t.as_str()), &mut self.tokens, parser);
    }

    #[inline]
    pub fn stringify(&self) -> String {
        let mut text = self.text.iter().map(|l| l.as_str()).collect::<Vec<_>>().join("\n");
        text.push('\n');
        text
    }
}

#[cfg(test)]
mod test {
    use std::{path::PathBuf, sync::Arc};

    use crate::{
        configs::FileType,
        lsp::{
            as_url,
            local::{tokens::python::PyToken, LangStream, Payload},
            LSPNotification, LSPResponse, LSPResponseType,
        },
    };

    use super::EnrichedLSP;
    use lsp_types::{
        notification::DidOpenTextDocument, Position, PositionEncodingKind, Range, SemanticToken,
        TextDocumentContentChangeEvent, Uri,
    };

    fn create_lsp<T: LangStream>(text: &str, encoding: Option<PositionEncodingKind>) -> (Uri, EnrichedLSP<T>) {
        let key = as_url(PathBuf::from("/home/test.py").as_path());
        let notification =
            LSPNotification::<DidOpenTextDocument>::file_did_open(key.to_owned(), FileType::Python, text.to_owned())
                .stringify()
                .unwrap();
        let mut enriched = EnrichedLSP::<T>::from_encoding(Arc::default(), encoding);
        enriched.pre_process(Payload::Direct(notification)).unwrap();
        (key, enriched)
    }

    #[test]
    fn test_utf32() {
        let (key, mut lsp) =
            create_lsp::<PyToken>("def main()\n    print(\"hello 🚀\")", Some(PositionEncodingKind::UTF32));

        let doc = lsp.documents.get(&key).unwrap();
        assert_eq!(doc.text, ["def main()", "    print(\"hello 🚀\")"]);

        lsp.pre_process(Payload::Tokens(key.to_owned(), 0)).unwrap();

        let full_tokens =
            match LSPResponseType::Tokens(0).parse(lsp.responses.lock().unwrap().remove(&0).unwrap().result).unwrap() {
                LSPResponse::Tokens(lsp_types::SemanticTokensResult::Tokens(data)) => data.data,
                _ => panic!("Expected Tokens response"),
            };

        assert_eq!(
            full_tokens,
            [
                SemanticToken { delta_line: 0, delta_start: 0, length: 3, token_type: 11, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 4, length: 4, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 1, delta_start: 4, length: 5, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 6, length: 9, token_type: 13, token_modifiers_bitset: 0 }
            ]
        );

        lsp.pre_process(Payload::Sync(
            key.to_owned(),
            0,
            vec![TextDocumentContentChangeEvent {
                text: String::from(" fast world"),
                range: Some(Range::new(Position::new(1, 18), Position::new(1, 18))),
                range_length: None,
            }],
        ))
        .unwrap();

        let doc = lsp.documents.get(&key).unwrap();
        assert_eq!(doc.text, ["def main()", "    print(\"hello 🚀 fast world\")"]);

        lsp.pre_process(Payload::PartialTokens(
            key.to_owned(),
            Range::new(Position::new(0, 4), Position::new(1, 18)),
            1,
        ))
        .unwrap();

        let partial_tokens = match (LSPResponseType::TokensPartial { id: 1, max_lines: 5 })
            .parse(lsp.responses.lock().unwrap().remove(&1).unwrap().result)
            .unwrap()
        {
            LSPResponse::TokensPartial { result: lsp_types::SemanticTokensRangeResult::Tokens(data), .. } => data.data,
            _ => panic!("Expected Tokens response"),
        };

        assert_eq!(
            partial_tokens,
            [
                SemanticToken { delta_line: 0, delta_start: 4, length: 4, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 1, delta_start: 4, length: 5, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 6, length: 20, token_type: 13, token_modifiers_bitset: 0 }
            ]
        );
    }

    #[test]
    fn test_utf16() {
        let (key, mut lsp) =
            create_lsp::<PyToken>("def main()\n    print(\"hello 🚀\")", Some(PositionEncodingKind::UTF16));

        let doc = lsp.documents.get(&key).unwrap();
        assert_eq!(doc.text, ["def main()", "    print(\"hello 🚀\")"]);

        lsp.pre_process(Payload::Tokens(key.to_owned(), 0)).unwrap();

        let full_tokens =
            match LSPResponseType::Tokens(0).parse(lsp.responses.lock().unwrap().remove(&0).unwrap().result).unwrap() {
                LSPResponse::Tokens(lsp_types::SemanticTokensResult::Tokens(data)) => data.data,
                _ => panic!("Expected Tokens response"),
            };

        assert_eq!(
            full_tokens,
            [
                SemanticToken { delta_line: 0, delta_start: 0, length: 3, token_type: 11, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 4, length: 4, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 1, delta_start: 4, length: 5, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 6, length: 10, token_type: 13, token_modifiers_bitset: 0 }
            ]
        );

        lsp.pre_process(Payload::Sync(
            key.to_owned(),
            0,
            vec![TextDocumentContentChangeEvent {
                text: String::from(" fast world"),
                range: Some(Range::new(Position::new(1, 19), Position::new(1, 19))),
                range_length: None,
            }],
        ))
        .unwrap();

        let doc = lsp.documents.get(&key).unwrap();
        assert_eq!(doc.text, ["def main()", "    print(\"hello 🚀 fast world\")"]);

        lsp.pre_process(Payload::PartialTokens(
            key.to_owned(),
            Range::new(Position::new(0, 4), Position::new(1, 18)),
            1,
        ))
        .unwrap();

        let partial_tokens = match (LSPResponseType::TokensPartial { id: 1, max_lines: 5 })
            .parse(lsp.responses.lock().unwrap().remove(&1).unwrap().result)
            .unwrap()
        {
            LSPResponse::TokensPartial { result: lsp_types::SemanticTokensRangeResult::Tokens(data), .. } => data.data,
            _ => panic!("Expected Tokens response"),
        };

        assert_eq!(
            partial_tokens,
            [
                SemanticToken { delta_line: 0, delta_start: 4, length: 4, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 1, delta_start: 4, length: 5, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 6, length: 21, token_type: 13, token_modifiers_bitset: 0 }
            ]
        );
    }

    #[test]
    fn test_utf8() {
        let (key, mut lsp) =
            create_lsp::<PyToken>("def main()\n    print(\"hello 🚀\")", Some(PositionEncodingKind::UTF8));

        let doc = lsp.documents.get(&key).unwrap();
        assert_eq!(doc.text, ["def main()", "    print(\"hello 🚀\")"]);

        lsp.pre_process(Payload::Tokens(key.to_owned(), 0)).unwrap();

        let full_tokens =
            match LSPResponseType::Tokens(0).parse(lsp.responses.lock().unwrap().remove(&0).unwrap().result).unwrap() {
                LSPResponse::Tokens(lsp_types::SemanticTokensResult::Tokens(data)) => data.data,
                _ => panic!("Expected Tokens response"),
            };

        assert_eq!(
            full_tokens,
            [
                SemanticToken { delta_line: 0, delta_start: 0, length: 3, token_type: 11, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 4, length: 4, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 1, delta_start: 4, length: 5, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 6, length: 12, token_type: 13, token_modifiers_bitset: 0 }
            ]
        );

        lsp.pre_process(Payload::Sync(
            key.to_owned(),
            0,
            vec![TextDocumentContentChangeEvent {
                text: String::from(" fast world"),
                range: Some(Range::new(Position::new(1, 21), Position::new(1, 21))),
                range_length: None,
            }],
        ))
        .unwrap();

        let doc = lsp.documents.get(&key).unwrap();
        assert_eq!(doc.text, ["def main()", "    print(\"hello 🚀 fast world\")"]);

        lsp.pre_process(Payload::PartialTokens(
            key.to_owned(),
            Range::new(Position::new(0, 4), Position::new(1, 18)),
            1,
        ))
        .unwrap();

        let partial_tokens = match (LSPResponseType::TokensPartial { id: 1, max_lines: 5 })
            .parse(lsp.responses.lock().unwrap().remove(&1).unwrap().result)
            .unwrap()
        {
            LSPResponse::TokensPartial { result: lsp_types::SemanticTokensRangeResult::Tokens(data), .. } => data.data,
            _ => panic!("Expected Tokens response"),
        };

        assert_eq!(
            partial_tokens,
            [
                SemanticToken { delta_line: 0, delta_start: 4, length: 4, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 1, delta_start: 4, length: 5, token_type: 10, token_modifiers_bitset: 0 },
                SemanticToken { delta_line: 0, delta_start: 6, length: 23, token_type: 13, token_modifiers_bitset: 0 }
            ]
        );
    }
}
