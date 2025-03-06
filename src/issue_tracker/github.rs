use async_trait::async_trait;
use reqwest::Method;
use serde::Deserialize;
use tracing::info;

use super::{IssueTrackerAdapter, Ticket, UpstreamError, builder::TrackerConfig};

pub struct Github {
    user: String,
    repo: String,
    client: reqwest::Client,
}

#[derive(Deserialize, Debug, Clone)]
struct ListIssuesResponse {
    number: u64,
    title: String,
    body: String,
}

impl Github {
    pub fn new(config: TrackerConfig) -> Option<Self> {
        let user = config.url.owner.clone()?;
        let repo = config.url.name.clone();
        info!("Created github instance for {user:?}@{repo}");
        Some(Self {
            user,
            repo,
            client: reqwest::Client::new(),
        })
    }
}

#[async_trait]
impl IssueTrackerAdapter for Github {
    async fn list_ticket_numbers(&self) -> Result<Vec<u64>, UpstreamError> {
        // url: https://api.github.com/repos/<user>/<repo>/issues?assignee=<user>
        let result = self
            .client
            .request(
                Method::GET,
                format!(
                    "https://api.github.com/repos/{0}/{1}/issues?assignee={0}",
                    self.user, self.repo
                ),
            )
            .header("User-Agent", "commit-lsp")
            .header("Accept", "application/vnd.github.raw+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await;

        // TODO(texel,2025-03-27): We get all the title and body information here but the current
        // API prevents us from returning it directly. The API could be refactored, so this
        // function can return Tickets or u64 to avoid redoing some requests.
        let response: Vec<ListIssuesResponse> = result.unwrap().json().await.unwrap();

        Ok(response.into_iter().map(|i| i.number).collect())
    }
    async fn get_ticket_details(&self, ids: &[u64]) -> Result<Vec<Ticket>, UpstreamError> {
        // url: https://api.github.com/repos/<user>/<repo>/issues/<id>
        let mut tickets = Vec::new();
        for id in ids {
            let result = self
                .client
                .request(
                    Method::GET,
                    format!(
                        "https://api.github.com/repos/{}/{}/issues/{}",
                        self.user, self.repo, id
                    ),
                )
                .header("User-Agent", "commit-lsp")
                .header("Accept", "application/vnd.github.raw+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await;

            let response: ListIssuesResponse = result.unwrap().json().await.unwrap();

            tickets.push(Ticket {
                id: *id,
                title: response.title,
                text: response.body,
            });
        }
        Ok(tickets)
    }
}
