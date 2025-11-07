use crate::document::DocStats;
use crate::index::InvertedIndex;
use std::collections::HashMap;

/// BM25 parameters
pub struct BM25 {
    k1: f64,
    b: f64,
}

impl Default for BM25 {
    fn default() -> Self {
        Self {
            k1: 1.5, // Term frequency saturation parameter
            b: 0.75, // Length normalization parameter
        }
    }
}

impl BM25 {
    pub fn new(k1: f64, b: f64) -> Self {
        Self { k1, b }
    }

    /// Calculate BM25 score for a document
    pub fn score(
        &self,
        query_terms: &[String],
        doc_stats: &DocStats,
        index: &InvertedIndex,
        avg_doc_length: f64,
    ) -> f64 {
        let mut score = 0.0;
        let doc_length = doc_stats.length as f64;
        let total_docs = index.total_documents() as f64;

        for term in query_terms {
            // Get term frequency in document
            let tf = *doc_stats.term_frequencies.get(term).unwrap_or(&0) as f64;

            if tf == 0.0 {
                continue;
            }

            // Calculate IDF (Inverse Document Frequency)
            let doc_freq = index.doc_frequency(term) as f64;
            let idf = if doc_freq > 0.0 {
                ((total_docs - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln()
            } else {
                0.0
            };

            // Calculate BM25 score component
            let normalized_tf =
                (tf * (self.k1 + 1.0)) / (tf + self.k1 * (1.0 - self.b + self.b * (doc_length / avg_doc_length)));

            score += idf * normalized_tf;
        }

        score
    }
}

/// Ranked search result
#[derive(Debug, Clone)]
pub struct ScoredDocument {
    pub doc_id: String,
    pub score: f64,
}

impl ScoredDocument {
    pub fn new(doc_id: String, score: f64) -> Self {
        Self { doc_id, score }
    }
}

/// Rank documents using BM25
pub fn rank_documents(
    query_terms: &[String],
    candidate_docs: &[String],
    doc_stats_map: &HashMap<String, DocStats>,
    index: &InvertedIndex,
    avg_doc_length: f64,
) -> Vec<ScoredDocument> {
    let bm25 = BM25::default();
    let mut scored_docs = Vec::new();

    for doc_id in candidate_docs {
        if let Some(doc_stats) = doc_stats_map.get(doc_id) {
            let score = bm25.score(query_terms, doc_stats, index, avg_doc_length);
            scored_docs.push(ScoredDocument::new(doc_id.clone(), score));
        }
    }

    // Sort by score descending
    scored_docs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    scored_docs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bm25_score() {
        let bm25 = BM25::default();
        let mut index = InvertedIndex::new();
        index.add_document("doc1", &["test".to_string()]);

        let mut doc_stats = DocStats::new("doc1".to_string(), 10);
        doc_stats.term_frequencies.insert("test".to_string(), 2);

        let score = bm25.score(&["test".to_string()], &doc_stats, &index, 10.0);
        assert!(score > 0.0);
    }
}
