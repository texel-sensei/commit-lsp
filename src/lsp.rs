use std::sync::{Arc, Mutex};

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, DidChangeTextDocumentParams, DidOpenTextDocumentParams, Documentation,
    Hover, HoverContents, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
    InitializedParams, MarkedString, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkDoneProgressOptions,
};

use tower_lsp::jsonrpc::Result;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::info;

use crate::analysis::{self, ItemKind};
use crate::issue_tracker::IssueTracker;
use crate::text_util::Ellipse as _;

struct Backend {
    client: Client,
    analysis: Mutex<analysis::State>,
    tracker: Option<Arc<IssueTracker>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(tower_lsp::lsp_types::CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec!["#".to_owned()]),
                    all_commit_characters: None,
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: None,
                    },
                    completion_item: None,
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "commit-lsp".to_owned(),
                version: Some("0.0.1".to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        if let Some(tracker) = &self.tracker {
            let tracker = tracker.clone();
            tokio::spawn(async move {
                // Retrieve list of tickets after initialization to fill ticket cache.
                let _ = tracker.request_ticket_information().await;
            });
        }
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
            diags = analysis
                .all_diagnostics()
                .into_iter()
                .map(|d| d.into())
                .collect();
        }
        self.client
            .publish_diagnostics(params.text_document.uri, diags, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let text = &params
            .content_changes
            .first()
            .expect("expected to get full document")
            .text;

        let diags;
        {
            let mut analysis = self.analysis.lock().unwrap();

            analysis.update_text(&text);
            diags = analysis
                .all_diagnostics()
                .into_iter()
                .map(|d| d.into())
                .collect();
        }
        self.client
            .publish_diagnostics(params.text_document.uri, diags, None)
            .await;
    }

    async fn hover(&self, par: HoverParams) -> Result<Option<Hover>> {
        info!("Hover request");
        let pos = par.text_document_position_params.position;

        let Some(item) = self.analysis.lock().unwrap().lookup(pos) else {
            return Ok(None);
        };

        info!(text=item.text, "Hovering @");

        match item.kind {
            ItemKind::Ty => {
                let analysis = self.analysis.lock().unwrap();
                let Some(info) = analysis.commit_type_info() else {
                    return Ok(None);
                };

                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(format!(
                        "# {}\n\n{}",
                        info.summary, info.description
                    ))),
                    range: Some(item.range),
                }));
            }
            ItemKind::Scope => {
                let analysis = self.analysis.lock().unwrap();
                let Some(info) = analysis.commit_scope_info() else {
                    return Ok(None);
                };

                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(format!(
                        "# {}\n\n{}",
                        info.summary, info.description
                    ))),
                    range: Some(item.range),
                }));
            }
            ItemKind::Ref(id) => {
                if let Some(tracker) = &self.tracker {
                    let ticket = tracker
                        .get_ticket_details(id)
                        .await
                        .expect("To connect to remote");

                    let text = ticket
                        .map(|t| format!("# {}\n\n{}", t.title(), t.text()))
                        .unwrap_or_else(|| format!("#{id} not found!"));

                    return Ok(Some(Hover {
                        contents: HoverContents::Scalar(MarkedString::String(text)),
                        range: Some(item.range),
                    }));
                }
            }
        }

        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(item.text)),
            range: Some(item.range),
        }))
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        let Some(remote) = &self.tracker else {
            return Ok(None);
        };
        let items: Vec<_> = remote
            .list_tickets()
            .iter()
            .map(|ticket| {
                let short_title = ticket.title().truncate_ellipse_with(20, "…");
                CompletionItem {
                    label: format!("#{}", ticket.id()),
                    detail: Some(ticket.title().to_owned()),
                    kind: Some(CompletionItemKind::REFERENCE),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: Some(short_title.into()),
                    }),
                    documentation: Some(Documentation::String(ticket.text().to_owned())),
                    ..Default::default()
                }
            })
            .collect();

        if items.is_empty() {
            return Ok(None);
        }

        Ok(Some(CompletionResponse::Array(items)))
    }
}

pub async fn run_stdio(analysis: analysis::State, remote: Option<IssueTracker>) {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        analysis: analysis.into(),
        tracker: remote.map(Arc::new),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
