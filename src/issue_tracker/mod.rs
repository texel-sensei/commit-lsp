use std::{collections::BTreeMap, sync::Mutex};

use async_trait::async_trait;

mod builder;
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
pub enum UpstreamError {}

impl std::fmt::Display for UpstreamError {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unreachable!()
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
