use std::{collections::BTreeMap, ffi::OsStr, process::Command, sync::Mutex};

use async_trait::async_trait;

mod azure;
mod demo;
mod gitlab;

use azure::AzureDevops;
use git_url_parse::GitUrl;
use secure_string::SecureString;

use crate::config;

use self::{demo::DemoAdapter, gitlab::Gitlab};

pub struct IssueTracker {
    remote: Box<dyn IssueTrackerAdapter>,
    ticket_cache: Mutex<BTreeMap<u64, Ticket>>,
}

impl IssueTracker {
    pub fn guess_from_remote(url: GitUrl, config: &config::User) -> Option<Self> {
        let cred_command = lookup_credential_command(&url.to_string(), &config)?;
        let adapter: Box<dyn IssueTrackerAdapter> = match url.host?.as_str() {
            #[cfg(debug_assertions)]
            _ if std::env::var("COMMIT_LSP_DEMO_FOLDER").is_ok() => {
                let folder = std::env::var("COMMIT_LSP_DEMO_FOLDER").unwrap();
                Box::new(DemoAdapter::new(folder.into()))
            }
            "ssh.dev.azure.com" | "dev.azure.com" => {
                let pat = get_credentials(&cred_command)?;
                Box::new(AzureDevops::new(pat, url.organization?, url.owner?))
            }
            host if host.contains("gitlab") => {
                let token = get_credentials(&cred_command)?;
                let project = format!("{}/{}", url.owner?, url.name);
                Box::new(Gitlab::new(token, host.to_owned(), project))
            }
            _ => return None,
        };

        Some(Self {
            remote: adapter,
            ticket_cache: Default::default(),
        })
    }

    pub async fn request_ticket_information(&self) -> Result<Vec<Ticket>, UpstreamError> {
        let ids = self.remote.list_ticket_numbers().await?;

        let tickets = self.remote.get_ticket_details(&ids).await?;

        self.ticket_cache
            .lock()
            .unwrap()
            .extend(tickets.iter().map(|t| (t.id(), t.clone())));

        Ok(tickets)
    }

    pub fn list_tickets(&self) -> Vec<Ticket> {
        self.ticket_cache
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    pub async fn get_ticket_details(&self, id: u64) -> Result<Option<Ticket>, UpstreamError> {
        if let Some(ticket) = self.ticket_cache.lock().unwrap().get(&id) {
            return Ok(Some(ticket.clone()));
        }

        let tickets = self.remote.get_ticket_details(&[id]).await?;

        let Some(ticket) = tickets.first() else {
            return Ok(None);
        };

        assert_eq!(ticket.id(), id);
        self.ticket_cache.lock().unwrap().insert(id, ticket.clone());

        Ok(Some(ticket.clone()))
    }
}

#[derive(Debug, Clone)]
pub struct Ticket {
    id: u64,
    title: String,
    text: String,
}

impl Ticket {
    pub(super) fn new(id: u64, title: String, text: String) -> Self {
        Self { id, title, text }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn title(&self) -> &str {
        self.title.as_ref()
    }

    pub fn text(&self) -> &str {
        self.text.as_ref()
    }
}

pub enum UpstreamError {}

#[async_trait]
trait IssueTrackerAdapter: Send + Sync {
    async fn list_ticket_numbers(&self) -> Result<Vec<u64>, UpstreamError>;

    /// Request additional detail (like title or description) for the given IDs from upstream.
    /// If any IDs are invalid, then they will not be included in the result Vec.
    async fn get_ticket_details(&self, ids: &[u64]) -> Result<Vec<Ticket>, UpstreamError>;
}

fn get_credentials(cmdline: &[impl AsRef<OsStr>]) -> Option<SecureString> {
    let pat = {
        let (cmd, args) = cmdline.split_first()?;

        let out = Command::new(cmd).args(args).output();
        String::from_utf8(out.unwrap().stdout)
            .unwrap()
            .trim()
            .into()
    };
    Some(pat)
}

fn lookup_credential_command(url: &str, config: &config::User) -> Option<Vec<String>> {
    config
        .remotes
        .iter()
        .find(|r| url.contains(&r.host))
        .map(|r| r.credentials_command.clone())
}
