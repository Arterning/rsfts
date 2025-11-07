use crate::document::{Document, DocStats};
use crate::index::InvertedIndex;
use crate::ranking::{rank_documents, ScoredDocument};
use crate::storage::Storage;
use crate::tokenizer::Tokenizer;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Search mode
#[derive(Debug, Clone, Copy)]
pub enum SearchMode {
    /// Match all query terms (AND)
    And,
    /// Match any query term (OR)
    Or,
}

/// Search options
#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub mode: SearchMode,
    pub use_ranking: bool,
    pub limit: Option<usize>,
    pub offset: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            mode: SearchMode::And,
            use_ranking: true,
            limit: Some(10),
            offset: 0,
        }
    }
}

/// Search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub documents: Vec<Document>,
    pub total: usize,
    pub scores: Option<Vec<f64>>,
}

/// Main search engine
pub struct SearchEngine {
    storage: Storage,
    index: Arc<RwLock<InvertedIndex>>,
    doc_stats: Arc<RwLock<HashMap<String, DocStats>>>,
    tokenizer: Tokenizer,
    avg_doc_length: Arc<RwLock<f64>>,
}

impl SearchEngine {
    /// Create a new search engine with storage path
    pub fn new(storage_path: &str) -> Result<Self> {
        let storage = Storage::open(storage_path)?;
        let tokenizer = Tokenizer::new();

        // Load or create index
        let index = storage.load_index()?.unwrap_or_else(InvertedIndex::new);

        // Load document statistics
        let stats_vec = storage.get_all_doc_stats()?;
        let doc_stats: HashMap<String, DocStats> =
            stats_vec.into_iter().map(|s| (s.id.clone(), s)).collect();

        // Calculate average document length
        let avg_doc_length = if doc_stats.is_empty() {
            0.0
        } else {
            doc_stats.values().map(|s| s.length).sum::<usize>() as f64 / doc_stats.len() as f64
        };

        Ok(Self {
            storage,
            index: Arc::new(RwLock::new(index)),
            doc_stats: Arc::new(RwLock::new(doc_stats)),
            tokenizer,
            avg_doc_length: Arc::new(RwLock::new(avg_doc_length)),
        })
    }

    /// Create an in-memory search engine (for testing)
    pub fn in_memory() -> Result<Self> {
        let storage = Storage::in_memory()?;
        let tokenizer = Tokenizer::new();

        Ok(Self {
            storage,
            index: Arc::new(RwLock::new(InvertedIndex::new())),
            doc_stats: Arc::new(RwLock::new(HashMap::new())),
            tokenizer,
            avg_doc_length: Arc::new(RwLock::new(0.0)),
        })
    }

    /// Insert or update a document
    pub fn upsert_document(&self, doc: Document) -> Result<()> {
        let doc_id = doc.id.clone();
        let searchable_text = doc.searchable_text();

        // Tokenize and analyze
        let tokens = self.tokenizer.analyze(&searchable_text);
        let term_frequencies = self.tokenizer.analyze_with_frequencies(&searchable_text);

        // Create document statistics
        let doc_stats = DocStats {
            id: doc_id.clone(),
            length: tokens.len(),
            term_frequencies,
        };

        // Update index
        {
            let mut index = self.index.write().unwrap();
            index.update_document(&doc_id, &tokens);

            // Persist index
            self.storage.save_index(&*index)?;
        }

        // Update document statistics
        {
            let mut stats_map = self.doc_stats.write().unwrap();
            stats_map.insert(doc_id.clone(), doc_stats.clone());

            // Recalculate average document length
            let avg = if stats_map.is_empty() {
                0.0
            } else {
                stats_map.values().map(|s| s.length).sum::<usize>() as f64 / stats_map.len() as f64
            };
            *self.avg_doc_length.write().unwrap() = avg;
        }

        // Save to storage
        self.storage.save_document(&doc)?;
        self.storage.save_doc_stats(&doc_stats)?;

        Ok(())
    }

    /// Batch insert documents (more efficient)
    pub fn batch_insert(&self, docs: Vec<Document>) -> Result<()> {
        for doc in docs {
            self.upsert_document(doc)?;
        }
        self.storage.flush()?;
        Ok(())
    }

    /// Delete a document
    pub fn delete_document(&self, doc_id: &str) -> Result<()> {
        // Remove from index
        {
            let mut index = self.index.write().unwrap();
            index.remove_document(doc_id);
            self.storage.save_index(&*index)?;
        }

        // Remove from statistics
        {
            let mut stats_map = self.doc_stats.write().unwrap();
            stats_map.remove(doc_id);

            // Recalculate average document length
            let avg = if stats_map.is_empty() {
                0.0
            } else {
                stats_map.values().map(|s| s.length).sum::<usize>() as f64 / stats_map.len() as f64
            };
            *self.avg_doc_length.write().unwrap() = avg;
        }

        // Remove from storage
        self.storage.delete_document(doc_id)?;
        self.storage.delete_doc_stats(doc_id)?;

        Ok(())
    }

    /// Get a document by ID
    pub fn get_document(&self, doc_id: &str) -> Result<Option<Document>> {
        self.storage.get_document(doc_id)
    }

    /// Search for documents
    pub fn search(&self, query: &str, options: &SearchOptions) -> Result<SearchResult> {
        // Tokenize query
        let query_tokens = self.tokenizer.analyze(query);

        if query_tokens.is_empty() {
            return Ok(SearchResult {
                documents: Vec::new(),
                total: 0,
                scores: None,
            });
        }

        // Find matching documents
        let candidate_ids = {
            let index = self.index.read().unwrap();
            match options.mode {
                SearchMode::And => index.search_and(&query_tokens),
                SearchMode::Or => index.search_or(&query_tokens),
            }
        };

        let total = candidate_ids.len();

        // Rank documents if requested
        let (sorted_ids, scores) = if options.use_ranking {
            let index = self.index.read().unwrap();
            let stats_map = self.doc_stats.read().unwrap();
            let avg_length = *self.avg_doc_length.read().unwrap();

            let scored_docs = rank_documents(&query_tokens, &candidate_ids, &stats_map, &*index, avg_length);

            let ids: Vec<String> = scored_docs.iter().map(|sd| sd.doc_id.clone()).collect();
            let scores: Vec<f64> = scored_docs.iter().map(|sd| sd.score).collect();

            (ids, Some(scores))
        } else {
            (candidate_ids, None)
        };

        // Apply pagination
        let start = options.offset;
        let end = if let Some(limit) = options.limit {
            (start + limit).min(sorted_ids.len())
        } else {
            sorted_ids.len()
        };

        let page_ids = &sorted_ids[start..end];
        let page_scores = scores.as_ref().map(|s| s[start..end].to_vec());

        // Fetch documents
        let mut documents = Vec::new();
        for id in page_ids {
            if let Some(doc) = self.storage.get_document(id)? {
                documents.push(doc);
            }
        }

        Ok(SearchResult {
            documents,
            total,
            scores: page_scores,
        })
    }

    /// Get index statistics
    pub fn stats(&self) -> Result<crate::index::IndexStats> {
        let index = self.index.read().unwrap();
        Ok(index.stats())
    }

    /// Get total document count
    pub fn document_count(&self) -> Result<usize> {
        self.storage.count_documents()
    }

    /// Flush all changes to disk
    pub fn flush(&self) -> Result<()> {
        self.storage.flush()
    }

    /// Clear all data
    pub fn clear(&self) -> Result<()> {
        {
            let mut index = self.index.write().unwrap();
            *index = InvertedIndex::new();
        }
        {
            let mut stats = self.doc_stats.write().unwrap();
            stats.clear();
        }
        self.storage.clear()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_insert_and_search() -> Result<()> {
        let engine = SearchEngine::in_memory()?;

        let doc1 = Document::new(
            "1".to_string(),
            "Rust Programming".to_string(),
            "Rust is a systems programming language".to_string(),
        );

        let doc2 = Document::new(
            "2".to_string(),
            "Go Programming".to_string(),
            "Go is a simple programming language".to_string(),
        );

        engine.upsert_document(doc1)?;
        engine.upsert_document(doc2)?;

        let results = engine.search("programming language", &SearchOptions::default())?;
        assert_eq!(results.total, 2);

        Ok(())
    }
}
