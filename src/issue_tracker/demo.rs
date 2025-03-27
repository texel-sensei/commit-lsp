use std::{fs::File, io::Read as _, path::PathBuf};

use async_trait::async_trait;

use super::{IssueTrackerAdapter, Ticket, UpstreamError};

pub struct DemoAdapter {
    source_folder: PathBuf,
}

impl DemoAdapter {
    pub fn new(source_folder: PathBuf) -> Self {
        Self { source_folder }
    }

    fn load_ticket(&self, id: u64) -> Option<Ticket> {
        let file = self.source_folder.join(id.to_string());

        let mut file = File::open(file).ok()?;
        let mut buffer = String::new();
        file.read_to_string(&mut buffer).ok()?;

        let mut lines = buffer.lines();
        let title = lines.next()?;
        let content = lines.skip(1).collect::<Vec<_>>().join("\n");

        Some(Ticket::new(id, title.to_owned(), content))
    }
}

#[async_trait]
impl IssueTrackerAdapter for DemoAdapter {
    async fn list_ticket_numbers(&self) -> Result<Vec<u64>, UpstreamError> {
        let mut ids = Vec::new();

        for entry in self.source_folder.read_dir().expect("To open dir") {
            let Ok(entry) = entry else {
                continue;
            };
            if let Some(i) = entry
                .path()
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.parse().ok())
            {
                ids.push(i)
            }
        }

        Ok(ids)
    }

    async fn get_ticket_details(&self, ids: &[u64]) -> Result<Vec<Ticket>, UpstreamError> {
        Ok(ids.iter().flat_map(|i| self.load_ticket(*i)).collect())
    }
}
