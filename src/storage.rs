use crate::document::{Document, DocStats};
use crate::index::InvertedIndex;
use anyhow::{Context, Result};
use sled::Db;
use std::path::Path;

const DOCS_TREE: &str = "documents";
const STATS_TREE: &str = "doc_stats";
const INDEX_TREE: &str = "index";
const METADATA_TREE: &str = "metadata";

pub struct Storage {
    db: Db,
}

impl Storage {
    /// Open or create a storage database
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db = sled::open(path).context("Failed to open database")?;
        Ok(Self { db })
    }

    /// Create an in-memory database (for testing)
    pub fn in_memory() -> Result<Self> {
        let config = sled::Config::new().temporary(true);
        let db = config.open().context("Failed to create in-memory database")?;
        Ok(Self { db })
    }

    // ========== Document Operations ==========

    /// Save a document
    pub fn save_document(&self, doc: &Document) -> Result<()> {
        let tree = self.db.open_tree(DOCS_TREE)?;
        let serialized = bincode::serialize(doc)?;
        tree.insert(doc.id.as_bytes(), serialized)?;
        Ok(())
    }

    /// Get a document by ID
    pub fn get_document(&self, id: &str) -> Result<Option<Document>> {
        let tree = self.db.open_tree(DOCS_TREE)?;
        if let Some(data) = tree.get(id.as_bytes())? {
            let doc: Document = bincode::deserialize(&data)?;
            Ok(Some(doc))
        } else {
            Ok(None)
        }
    }

    /// Delete a document
    pub fn delete_document(&self, id: &str) -> Result<()> {
        let tree = self.db.open_tree(DOCS_TREE)?;
        tree.remove(id.as_bytes())?;
        Ok(())
    }

    /// Get all documents
    pub fn get_all_documents(&self) -> Result<Vec<Document>> {
        let tree = self.db.open_tree(DOCS_TREE)?;
        let mut docs = Vec::new();

        for item in tree.iter() {
            let (_, value) = item?;
            let doc: Document = bincode::deserialize(&value)?;
            docs.push(doc);
        }

        Ok(docs)
    }

    /// Count total documents
    pub fn count_documents(&self) -> Result<usize> {
        let tree = self.db.open_tree(DOCS_TREE)?;
        Ok(tree.len())
    }

    // ========== Document Statistics Operations ==========

    /// Save document statistics
    pub fn save_doc_stats(&self, stats: &DocStats) -> Result<()> {
        let tree = self.db.open_tree(STATS_TREE)?;
        let serialized = bincode::serialize(stats)?;
        tree.insert(stats.id.as_bytes(), serialized)?;
        Ok(())
    }

    /// Get document statistics
    pub fn get_doc_stats(&self, id: &str) -> Result<Option<DocStats>> {
        let tree = self.db.open_tree(STATS_TREE)?;
        if let Some(data) = tree.get(id.as_bytes())? {
            let stats: DocStats = bincode::deserialize(&data)?;
            Ok(Some(stats))
        } else {
            Ok(None)
        }
    }

    /// Get all document statistics
    pub fn get_all_doc_stats(&self) -> Result<Vec<DocStats>> {
        let tree = self.db.open_tree(STATS_TREE)?;
        let mut stats = Vec::new();

        for item in tree.iter() {
            let (_, value) = item?;
            let doc_stats: DocStats = bincode::deserialize(&value)?;
            stats.push(doc_stats);
        }

        Ok(stats)
    }

    /// Delete document statistics
    pub fn delete_doc_stats(&self, id: &str) -> Result<()> {
        let tree = self.db.open_tree(STATS_TREE)?;
        tree.remove(id.as_bytes())?;
        Ok(())
    }

    // ========== Index Operations ==========

    /// Save the inverted index
    pub fn save_index(&self, index: &InvertedIndex) -> Result<()> {
        let tree = self.db.open_tree(INDEX_TREE)?;
        let serialized = bincode::serialize(index)?;
        tree.insert(b"main_index", serialized)?;
        tree.flush()?;
        Ok(())
    }

    /// Load the inverted index
    pub fn load_index(&self) -> Result<Option<InvertedIndex>> {
        let tree = self.db.open_tree(INDEX_TREE)?;
        if let Some(data) = tree.get(b"main_index")? {
            let index: InvertedIndex = bincode::deserialize(&data)?;
            Ok(Some(index))
        } else {
            Ok(None)
        }
    }

    // ========== Metadata Operations ==========

    /// Save metadata (e.g., average document length)
    pub fn save_metadata(&self, key: &str, value: &str) -> Result<()> {
        let tree = self.db.open_tree(METADATA_TREE)?;
        tree.insert(key.as_bytes(), value.as_bytes())?;
        Ok(())
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Result<Option<String>> {
        let tree = self.db.open_tree(METADATA_TREE)?;
        if let Some(data) = tree.get(key.as_bytes())? {
            Ok(Some(String::from_utf8(data.to_vec())?))
        } else {
            Ok(None)
        }
    }

    /// Flush all changes to disk
    pub fn flush(&self) -> Result<()> {
        self.db.flush()?;
        Ok(())
    }

    /// Clear all data
    pub fn clear(&self) -> Result<()> {
        self.db.drop_tree(DOCS_TREE)?;
        self.db.drop_tree(STATS_TREE)?;
        self.db.drop_tree(INDEX_TREE)?;
        self.db.drop_tree(METADATA_TREE)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_document() -> Result<()> {
        let storage = Storage::in_memory()?;
        let doc = Document::new("1".to_string(), "Test".to_string(), "Content".to_string());

        storage.save_document(&doc)?;
        let loaded = storage.get_document("1")?;

        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().title, "Test");

        Ok(())
    }
}
