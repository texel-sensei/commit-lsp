use std::sync::OnceLock;

use async_trait::async_trait;
use reqwest::Method;
use secure_string::SecureString;
use serde::Deserialize;
use tracing::info;

use super::{IssueTrackerAdapter, Ticket, UpstreamError, builder::TrackerConfig};

pub struct Github {
    username: OnceLock<String>,
    token: Option<SecureString>,
    owner: String,
    repo: String,
    client: reqwest::Client,
}

#[derive(Deserialize, Debug, Clone)]
struct ListIssuesResponse {
    number: u64,
    title: String,
    body: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct UserResponse {
    login: String,
}

impl Github {
    pub fn new(config: TrackerConfig) -> Option<Self> {
        let owner = config.url.owner.clone()?;
        let repo = config.url.name.clone();
        let secret = config.secret.clone();
        info!("Created github instance for {owner:?}@{repo}");
        Some(Self {
            username: OnceLock::new(),
            token: secret,
            owner,
            repo,
            client: reqwest::Client::new(),
        })
    }

    async fn get_username(&self) -> Result<String, UpstreamError> {
        let Some(token) = &self.token else {
            return Ok(self.owner.clone());
        };
        if let Some(username) = self.username.get() {
            return Ok(username.into());
        }
        let response = self
            .client
            .get("https://api.github.com/user")
            .header("User-Agent", "commit-lsp")
            .header("Accept", "application/vnd.github.raw+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Authorization", format!("Bearer {}", token.unsecure()))
            .send()
            .await?;
        let user = response.json::<UserResponse>().await?;
        let username = user.login;
        self.username
            .set(username.clone())
            .map_err(|_| UpstreamError::Other("Failed to set username".into()))?;
        Ok(username)
    }
}

#[async_trait]
impl IssueTrackerAdapter for Github {
    async fn list_ticket_numbers(&self) -> Result<Vec<u64>, UpstreamError> {
        // url: https://api.github.com/repos/<user>/<repo>/issues?assignee=<user>
        let assignee = self.get_username().await?;
        let mut builder = self.client.request(
            Method::GET,
            format!(
                "https://api.github.com/repos/{0}/{1}/issues?assignee={2}",
                self.owner, self.repo, assignee
            ),
        );
        if let Some(token) = &self.token {
            builder = builder.header("Authorization", format!("Bearer {}", token.unsecure()));
        }
        let result = builder
            .header("User-Agent", "commit-lsp")
            .header("Accept", "application/vnd.github.raw+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await?;

        // TODO(texel,2025-03-27): We get all the title and body information here but the current
        // API prevents us from returning it directly. The API could be refactored, so this
        // function can return Tickets or u64 to avoid redoing some requests.
        if !result.status().is_success() {
            return Err(UpstreamError::Other(result.text().await?));
        }

        let response: Vec<ListIssuesResponse> = result.json().await?;

        Ok(response.into_iter().map(|i| i.number).collect())
    }
    async fn get_ticket_details(&self, ids: &[u64]) -> Result<Vec<Ticket>, UpstreamError> {
        // url: https://api.github.com/repos/<user>/<repo>/issues/<id>
        let mut tickets = Vec::new();
        for id in ids {
            let mut builder = self.client.request(
                Method::GET,
                format!(
                    "https://api.github.com/repos/{}/{}/issues/{}",
                    self.owner, self.repo, id
                ),
            );
            if let Some(token) = &self.token {
                builder = builder.header("Authorization", format!("Bearer {}", token.unsecure()));
            }

            let result = builder
                .header("User-Agent", "commit-lsp")
                .header("Accept", "application/vnd.github.raw+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .send()
                .await;

            let response: ListIssuesResponse = result?.json().await?;

            tickets.push(Ticket {
                id: *id,
                title: response.title,
                text: response.body.unwrap_or_default(),
            });
        }
        Ok(tickets)
    }
}
