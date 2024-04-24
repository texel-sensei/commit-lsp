use git_url_parse::GitUrl;
use std::process::Command;
use std::sync::{Arc, Mutex};

use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    CompletionResponse, DidChangeTextDocumentParams, DidOpenTextDocumentParams, Documentation,
    Hover, HoverContents, HoverParams, InitializeParams, InitializeResult, InitializedParams,
    MarkedString, ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkDoneProgressOptions,
};

use tower_lsp::jsonrpc::Result;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use crate::analysis;
use crate::issue_tracker::AzureDevops;
use crate::text_util::Ellipse as _;

struct Backend {
    client: Client,
    analysis: Mutex<analysis::State>,
    tracker: Option<Arc<AzureDevops>>,
    items: Arc<Mutex<Vec<(i64, (String, String))>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(tower_lsp::lsp_types::HoverProviderCapability::Simple(cfg!(debug_assertions))),
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
            let items = self.items.clone();
            tokio::spawn(async move {
                let resolved = tracker.get_work_items().await;
                *items.lock().unwrap() = resolved;
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
        let pos = par.text_document_position_params.position;

        let Some(item) = self.analysis.lock().unwrap().lookup(pos) else {
            return Ok(None);
        };

        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(item.text)),
            range: Some(item.range),
        }))
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        let items: Vec<_> = self
            .items
            .lock()
            .unwrap()
            .iter()
            .map(|(id, (title, desc))| {
                let short_title = title.as_str().truncate_ellipse_with(20, "…");
                CompletionItem {
                    label: format!("#{id}"),
                    detail: Some(title.clone()),
                    kind: Some(CompletionItemKind::REFERENCE),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: Some(short_title.into()),
                    }),
                    documentation: Some(Documentation::String(desc.clone())),
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

pub async fn run_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        analysis: Default::default(),
        tracker: initialize_issue_tracker().map(Arc::new),
        items: Default::default(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

fn initialize_issue_tracker() -> Option<AzureDevops> {
    let url_info = guess_repo_url()?;
    let organization = url_info.organization?;
    let project = url_info.owner?;
    let cred_command = std::env::var("COMMIT_LSP_CREDENTIAL_COMMAND").ok()?;
    let pat = {
        let cmdline: Vec<&str> = cred_command.split_whitespace().collect();
        let (cmd, args) = cmdline.split_first()?;

        let out = Command::new(cmd).args(args).output();
        String::from_utf8(out.unwrap().stdout)
            .unwrap()
            .trim()
            .into()
    };
    Some(AzureDevops::new(pat, organization, project))
}

fn guess_repo_url() -> Option<GitUrl> {
    let url = Command::new("git")
        .args(["ls-remote", "--get-url", "origin"])
        .output()
        .unwrap()
        .stdout;
    let url = String::from_utf8(url).unwrap();

    GitUrl::parse(url.trim()).ok()
}
