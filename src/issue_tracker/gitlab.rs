use async_trait::async_trait;
use gitlab::api::{issues::IssueState, AsyncQuery};
use secure_string::SecureString;
use serde::Deserialize;
use tokio::sync::OnceCell;

use super::{IssueTrackerAdapter, Ticket, UpstreamError};

pub struct Gitlab {
    client: OnceCell<gitlab::AsyncGitlab>,
    host: String,
    token: SecureString,
    project: String,
}

impl Gitlab {
    pub fn new(token: SecureString, host: String, project: String) -> Self {
        Self {
            client: Default::default(),
            host,
            token,
            project,
        }
    }

    async fn client(&self) -> Result<&gitlab::AsyncGitlab, UpstreamError> {
        self.client
            .get_or_try_init(|| async {
                Ok(
                    gitlab::GitlabBuilder::new(&self.host, self.token.unsecure())
                        .build_async()
                        .await
                        .expect("Failed to connect to gitlab"),
                )
            })
            .await
    }
}

#[async_trait]
impl IssueTrackerAdapter for Gitlab {
    async fn list_ticket_numbers(&self) -> Result<Vec<u64>, UpstreamError> {
        let request = gitlab::api::issues::ProjectIssues::builder()
            .state(IssueState::Opened)
            .project(&self.project)
            .build()
            .expect("Failed to build request");

        let issues: Vec<Issue> = request
            .query_async(self.client().await?)
            .await
            .expect("Expected to get issues");

        Ok(issues.into_iter().map(|i| i.iid).collect())
    }

    async fn get_ticket_details(&self, ids: &[u64]) -> Result<Vec<Ticket>, UpstreamError> {
        let request = gitlab::api::issues::ProjectIssues::builder()
            .iids(ids.iter().copied())
            .project(&self.project)
            .build()
            .expect("Failed to build request");

        let issues: Vec<Issue> = request
            .query_async(self.client().await?)
            .await
            .expect("Expected to get issues");

        Ok(issues
            .into_iter()
            .map(|i| Ticket::new(i.iid, i.title, i.description))
            .collect())
    }
}

#[derive(Deserialize, Clone, Debug)]
struct Issue {
    iid: u64,
    title: String,
    description: String,
}
