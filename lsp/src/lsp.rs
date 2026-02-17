use std::collections::HashMap;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct HttpLsp {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
}

const HTTP_METHODS: &[&str] = &[
    "GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS", "TRACE", "CONNECT",
];

const COMMON_HEADERS: &[(&str, &str)] = &[
    ("Accept", "application/json"),
    ("Accept-Encoding", "gzip, deflate, br"),
    ("Accept-Language", "en-US,en;q=0.9"),
    ("Authorization", "Bearer "),
    ("Cache-Control", "no-cache"),
    ("Content-Type", "application/json"),
    ("Content-Type", "application/x-www-form-urlencoded"),
    ("Content-Type", "text/plain"),
    ("Cookie", ""),
    ("Host", ""),
    ("Origin", ""),
    ("Referer", ""),
    ("User-Agent", ""),
    ("X-Request-ID", ""),
];

#[tower_lsp::async_trait]
impl LanguageServer for HttpLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec!["\n".into(), " ".into()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "zhttp-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.lock().unwrap().insert(uri, text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            self.documents.lock().unwrap().insert(uri, change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .lock()
            .unwrap()
            .remove(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        if !uri.path().ends_with(".http") {
            return Ok(None);
        }

        let line = params.text_document_position.position.line as usize;
        let character = params.text_document_position.position.character;

        let line_text = self.get_line(&uri, line);

        let items = if character == 0 || is_method_position(&line_text) {
            method_completions()
        } else {
            header_completions()
        };

        Ok(Some(CompletionResponse::Array(items)))
    }
}

impl HttpLsp {
    fn get_line(&self, uri: &Url, line: usize) -> String {
        let docs = self.documents.lock().unwrap();
        docs.get(uri)
            .and_then(|text| text.lines().nth(line))
            .unwrap_or("")
            .to_string()
    }
}

fn is_method_position(line_text: &str) -> bool {
    let trimmed = line_text.trim();
    if trimmed.is_empty() {
        return true;
    }
    let upper = trimmed.to_uppercase();
    HTTP_METHODS.iter().any(|m| upper.starts_with(m))
}

fn method_completions() -> Vec<CompletionItem> {
    HTTP_METHODS
        .iter()
        .enumerate()
        .map(|(i, method)| CompletionItem {
            label: method.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some("HTTP Method".into()),
            sort_text: Some(format!("{:02}", i)),
            insert_text: Some(format!("{} ", method)),
            ..Default::default()
        })
        .collect()
}

fn header_completions() -> Vec<CompletionItem> {
    COMMON_HEADERS
        .iter()
        .enumerate()
        .map(|(i, (name, default_value))| {
            let insert = if default_value.is_empty() {
                format!("{}: ", name)
            } else {
                format!("{}: {}", name, default_value)
            };
            CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some("HTTP Header".into()),
                sort_text: Some(format!("{:02}", i)),
                insert_text: Some(insert),
                ..Default::default()
            }
        })
        .collect()
}

pub fn start() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to create tokio runtime")
        .block_on(async {
            let stdin = tokio::io::stdin();
            let stdout = tokio::io::stdout();

            let (service, socket) = LspService::new(|client| HttpLsp {
                client,
                documents: Mutex::new(HashMap::new()),
            });
            Server::new(stdin, stdout, socket).serve(service).await;
        });
}
