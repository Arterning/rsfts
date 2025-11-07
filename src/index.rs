use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Inverted index: token -> list of document IDs
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InvertedIndex {
    index: HashMap<String, Vec<String>>,
    doc_count: usize,
}

impl InvertedIndex {
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
            doc_count: 0,
        }
    }

    /// Add a document to the index
    pub fn add_document(&mut self, doc_id: &str, tokens: &[String]) {
        let unique_tokens: HashSet<_> = tokens.iter().collect();

        for token in unique_tokens {
            let doc_list = self.index.entry(token.clone()).or_insert_with(Vec::new);

            // Only add if not already present
            if !doc_list.contains(&doc_id.to_string()) {
                doc_list.push(doc_id.to_string());
            }
        }

        self.doc_count += 1;
    }

    /// Remove a document from the index
    pub fn remove_document(&mut self, doc_id: &str) {
        for doc_list in self.index.values_mut() {
            doc_list.retain(|id| id != doc_id);
        }
        self.doc_count = self.doc_count.saturating_sub(1);

        // Clean up empty entries
        self.index.retain(|_, docs| !docs.is_empty());
    }

    /// Update a document (remove old, add new)
    pub fn update_document(&mut self, doc_id: &str, tokens: &[String]) {
        self.remove_document(doc_id);
        self.add_document(doc_id, tokens);
    }

    /// Get document IDs containing a token
    pub fn get_documents(&self, token: &str) -> Option<&Vec<String>> {
        self.index.get(token)
    }

    /// Get number of documents containing a term (for IDF calculation)
    pub fn doc_frequency(&self, token: &str) -> usize {
        self.index.get(token).map(|docs| docs.len()).unwrap_or(0)
    }

    /// Get total number of indexed documents
    pub fn total_documents(&self) -> usize {
        self.doc_count
    }

    /// Search for documents matching ALL tokens (AND query)
    pub fn search_and(&self, tokens: &[String]) -> Vec<String> {
        if tokens.is_empty() {
            return Vec::new();
        }

        let mut result: Option<HashSet<String>> = None;

        for token in tokens {
            if let Some(docs) = self.get_documents(token) {
                let docs_set: HashSet<String> = docs.iter().cloned().collect();

                result = Some(match result {
                    None => docs_set,
                    Some(r) => r.intersection(&docs_set).cloned().collect(),
                });
            } else {
                // Token not found, no results
                return Vec::new();
            }
        }

        result.unwrap_or_default().into_iter().collect()
    }

    /// Search for documents matching ANY token (OR query)
    pub fn search_or(&self, tokens: &[String]) -> Vec<String> {
        let mut result: HashSet<String> = HashSet::new();

        for token in tokens {
            if let Some(docs) = self.get_documents(token) {
                result.extend(docs.iter().cloned());
            }
        }

        result.into_iter().collect()
    }

    /// Get all tokens in the index
    pub fn all_tokens(&self) -> Vec<&String> {
        self.index.keys().collect()
    }

    /// Get index statistics
    pub fn stats(&self) -> IndexStats {
        IndexStats {
            total_documents: self.doc_count,
            total_tokens: self.index.len(),
            avg_docs_per_token: if self.index.is_empty() {
                0.0
            } else {
                self.index.values().map(|v| v.len()).sum::<usize>() as f64 / self.index.len() as f64
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IndexStats {
    pub total_documents: usize,
    pub total_tokens: usize,
    pub avg_docs_per_token: f64,
}
