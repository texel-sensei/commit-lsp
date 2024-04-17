use reqwest::Method;
use secure_string::SecureString;
use serde::Serialize;

pub struct AzureDevops {
    pat: SecureString,
    organization: String,
    project: String,
    client: reqwest::Client,
}

impl AzureDevops {
    pub fn new(pat: SecureString, organization: String, project: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            pat,
            organization,
            project,
        }
    }

    pub async fn get_work_items(&self) -> Vec<(i64, (String, String))> {
        let query = "SELECT [System.Id] FROM WorkItems WHERE [System.TeamProject] = @project AND [Assigned To] = @me AND [System.Id] in (@MyRecentActivity)".to_owned();
        let result = self
            .client
            .request(Method::POST, format!("{}/wit/wiql", self.base_url()))
            .query(&[("api-version", "7.0")])
            .json(&QueryRequest { query })
            .basic_auth("", Some(self.pat.unsecure()))
            .send()
            .await;

        let response: serde_json::Value = result.unwrap().json().await.unwrap();

        let items: Vec<_> = response["workItems"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["id"].as_i64().unwrap())
            .collect();

        let titles = self.resolve_item_titles(&items).await;

        items.into_iter().zip(titles.into_iter()).collect()
    }

    fn base_url(&self) -> String {
        format!(
            "https://dev.azure.com/{}/{}/_apis",
            self.organization, self.project
        )
    }

    async fn resolve_item_titles(&self, ids: &[i64]) -> Vec<(String, String)> {
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

        let response: serde_json::Value = result.unwrap().json().await.unwrap();
        let items: Vec<_> = response["value"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| {
                (
                    i["fields"]["System.Title"].as_str().unwrap().to_owned(),
                    i["fields"]["System.Description"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                )
            })
            .collect();

        items
    }
}

#[derive(Serialize)]
struct QueryRequest {
    pub query: String,
}

#[derive(Serialize)]
struct WorkItemsBatchRequest<'a> {
    pub ids: &'a [i64],
    pub fields: &'a [&'static str],
}
