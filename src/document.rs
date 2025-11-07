use serde::{Deserialize, Serialize};

/// Document represents a searchable document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,
}

impl Document {
    pub fn new(id: String, title: String, content: String) -> Self {
        Self {
            id,
            title,
            content,
            url: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Get the full searchable text (title + content)
    pub fn searchable_text(&self) -> String {
        format!("{} {}", self.title, self.content)
    }
}

/// Document statistics for BM25 ranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocStats {
    pub id: String,
    pub length: usize,
    pub term_frequencies: std::collections::HashMap<String, usize>,
}

impl DocStats {
    pub fn new(id: String, length: usize) -> Self {
        Self {
            id,
            length,
            term_frequencies: std::collections::HashMap::new(),
        }
    }
}
