use async_trait::async_trait;
use reqwest::{Method, StatusCode};
use secure_string::SecureString;
use serde::Serialize;

use super::{IssueTrackerAdapter, Ticket, UpstreamError, builder::TrackerConfig};

pub struct AzureDevops {
    pat: SecureString,
    organization: String,
    project: String,
    client: reqwest::Client,
}

impl AzureDevops {
    pub fn new(config: TrackerConfig) -> Option<Self> {
        Some(Self {
            client: reqwest::Client::new(),
            pat: config.secret?,
            organization: config.url.organization?,
            project: config.url.owner?,
        })
    }

    fn base_url(&self) -> String {
        format!(
            "https://dev.azure.com/{}/{}/_apis",
            self.organization, self.project
        )
    }
}

#[async_trait]
impl IssueTrackerAdapter for AzureDevops {
    async fn list_ticket_numbers(&self) -> Result<Vec<u64>, UpstreamError> {
        let query = "SELECT [System.Id] FROM WorkItems WHERE [System.TeamProject] = @project AND [Assigned To] = @me AND [System.Id] in (@MyRecentActivity)".to_owned();
        let response = self
            .client
            .request(Method::POST, format!("{}/wit/wiql", self.base_url()))
            .query(&[("api-version", "7.0")])
            .json(&QueryRequest { query })
            .header("Accept", "application/json")
            .basic_auth("", Some(self.pat.unsecure()))
            .send()
            .await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(UpstreamError::Authentication);
        }


        let response: serde_json::Value = response.json().await?;

        let items: Vec<_> = response["workItems"]
            .as_array()
            .ok_or(UpstreamError::Other(
                "Unexpected response format".to_owned(),
            ))?
            .iter()
            .map(|i| {
                i["id"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("Got ID that is not an integer! ({})", i["id"]))
            })
            .collect();

        Ok(items)
    }

    async fn get_ticket_details(&self, ids: &[u64]) -> Result<Vec<Ticket>, UpstreamError> {
        let result = self
            .client
            .request(
                Method::POST,
                format!("{}/wit/workitemsbatch", self.base_url()),
            )
            .json(&WorkItemsBatchRequest {
                ids,
                fields: &["System.Title", "System.Description"],
            })
            .query(&[("api-version", "7.0")])
            .basic_auth("", Some(self.pat.unsecure()))
            .send()
            .await;

        let response: serde_json::Value = result?.json().await?;
        let items: Vec<_> = response["value"]
            .as_array()
            .ok_or(UpstreamError::Other(
                "Unexpected response format".to_owned(),
            ))?
            .iter()
            .map(|i| {
                Ticket::new(
                    i["id"].as_u64().expect("Item ID is not an integer"),
                    i["fields"]["System.Title"]
                        .as_str()
                        .expect("Item is missing a title")
                        .to_owned(),
                    i["fields"]["System.Description"]
                        .as_str()
                        // We need to handle the case where a work item has no description,
                        // so we just default to empty string.
                        .unwrap_or_default()
                        .to_owned(),
                )
            })
            .collect();

        Ok(items)
    }
}

#[derive(Serialize)]
struct QueryRequest {
    pub query: String,
}

#[derive(Serialize)]
struct WorkItemsBatchRequest<'a> {
    pub ids: &'a [u64],
    pub fields: &'a [&'static str],
}
