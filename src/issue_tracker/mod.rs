use std::string::FromUtf8Error;
use std::{collections::BTreeMap, sync::Mutex};

use async_trait::async_trait;

mod builder;
use git_url_parse::GitUrlParseError;
use ::gitlab::GitlabError;
use ::gitlab::RestError;
use ::gitlab::api::ApiError;
pub use builder::Builder;
pub use builder::IssueTrackerType;

mod azure;
mod demo;
mod github;
mod gitlab;

pub struct IssueTracker {
    remote: Box<dyn IssueTrackerAdapter>,
    ticket_cache: Mutex<BTreeMap<u64, Ticket>>,
}

impl IssueTracker {
    fn new(adapter: Box<dyn IssueTrackerAdapter>) -> Self {
        Self {
            remote: adapter,
            ticket_cache: Default::default(),
        }
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

#[derive(Debug)]
pub enum UpstreamError {
    /// Input/Output with the remote failed (e.g. no internet connection).
    Io(std::io::Error),

    /// Authentication failed
    Authentication,

    /// Unspecified other errors.
    Other(String),
}

impl std::fmt::Display for UpstreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use UpstreamError as U;
        match self {
            U::Io(underlying) => write!(f, "IO Error interacting with remote: {underlying}"),
            U::Authentication => write!(f, "Authentication failed"),
            U::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl From<std::io::Error> for UpstreamError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<GitUrlParseError> for UpstreamError {
    fn from(value: GitUrlParseError) -> Self {
        Self::Other(value.to_string())
    }
}

impl From<FromUtf8Error> for UpstreamError {
    fn from(value: FromUtf8Error) -> Self {
        Self::Other(value.to_string())
    }
}

impl From<reqwest::Error> for UpstreamError {
    fn from(value: reqwest::Error) -> Self {
        Self::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            value.without_url(),
        ))
    }
}

impl From<ApiError<RestError>> for UpstreamError {
    fn from(value: ApiError<RestError>) -> Self {
        match value {
            ApiError::Client {
                source: RestError::AuthError { .. },
            } => Self::Authentication,
            ApiError::Auth { .. } => Self::Authentication,
            ApiError::Client {
                source: RestError::Communication { source },
            } => source.into(),
            _ => Self::Other(value.to_string()),
        }
    }
}

impl From<GitlabError> for UpstreamError {
    fn from(value: GitlabError) -> Self {
        match value {
            GitlabError::AuthError { .. } => Self::Authentication,
            GitlabError::Communication { source } => source.into(),
            GitlabError::Api { source } => source.into(),
            _ => Self::Other(value.to_string()),
        }
    }
}

impl std::error::Error for UpstreamError {}

#[async_trait]
trait IssueTrackerAdapter: Send + Sync {
    async fn list_ticket_numbers(&self) -> Result<Vec<u64>, UpstreamError>;

    /// Request additional detail (like title or description) for the given IDs from upstream.
    /// If any IDs are invalid, then they will not be included in the result Vec.
    async fn get_ticket_details(&self, ids: &[u64]) -> Result<Vec<Ticket>, UpstreamError>;
}
