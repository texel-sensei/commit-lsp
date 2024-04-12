use std::sync::Mutex;

use tower_lsp::lsp_types::{
    DidOpenTextDocumentParams, Hover, HoverContents, HoverParams, InitializeParams,
    InitializeResult, InitializedParams, MarkedString, MessageType, ServerCapabilities, ServerInfo,
    TextDocumentSyncCapability, TextDocumentSyncKind, DidChangeTextDocumentParams,
};

use tower_lsp::jsonrpc::Result;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::analysis;

struct Backend {
    client: Client,
    analysis: Mutex<analysis::State>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "commit-lsp".to_owned(),
                version: Some("0.0.1".to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .show_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let text = params.text_document.text;

        let diags;
        {
            let mut analysis = self.analysis.lock().unwrap();

            analysis.update_text(&text);
            diags = analysis.all_diagnostics().into_iter().map(|d| d.into()).collect();
        }
        self.client.publish_diagnostics(params.text_document.uri, diags, None).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let text = &params.content_changes.first().expect("expected to get full document").text;

        let diags;
        {
            let mut analysis = self.analysis.lock().unwrap();

            analysis.update_text(&text);
            diags = analysis.all_diagnostics().into_iter().map(|d| d.into()).collect();
        }
        self.client.publish_diagnostics(params.text_document.uri, diags, None).await;

    }

    async fn hover(&self, _: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("You're hovering!".to_string())),
            range: None,
        }))
    }
}

pub async fn run_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        analysis: Default::default(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
