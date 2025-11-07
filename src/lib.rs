// Re-export main components
pub mod api;
pub mod document;
pub mod engine;
pub mod index;
pub mod ranking;
pub mod storage;
pub mod tokenizer;

// Re-export commonly used types
pub use document::Document;
pub use engine::{SearchEngine, SearchMode, SearchOptions, SearchResult};
pub use index::InvertedIndex;
pub use storage::Storage;
pub use tokenizer::Tokenizer;

// Re-export error types
pub use anyhow::{Error, Result};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_workflow() -> Result<()> {
        let engine = SearchEngine::in_memory()?;

        // Insert document
        let doc = Document::new(
            "1".to_string(),
            "Rust Programming Language".to_string(),
            "Rust is a blazingly fast and memory-efficient language".to_string(),
        );

        engine.upsert_document(doc)?;

        // Search
        let results = engine.search("rust programming", &SearchOptions::default())?;

        assert_eq!(results.total, 1);
        assert!(!results.documents.is_empty());

        Ok(())
    }
}
