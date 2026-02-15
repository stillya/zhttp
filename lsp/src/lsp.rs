use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct HttpLsp {
    client: Client,
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
    ("Content-Type", "multipart/form-data"),
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

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        if !uri.path().ends_with(".http") {
            return Ok(None);
        }

        let line = params.text_document_position.position.line;
        let character = params.text_document_position.position.character;

        let items = if character == 0 || is_start_of_request_line(line, character) {
            method_completions()
        } else {
            header_completions()
        };

        Ok(Some(CompletionResponse::Array(items)))
    }
}

fn is_start_of_request_line(_line: u32, character: u32) -> bool {
    character < 10
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

            let (service, socket) = LspService::new(|client| HttpLsp { client });
            Server::new(stdin, stdout, socket).serve(service).await;
        });
}
