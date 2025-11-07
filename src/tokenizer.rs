use rust_stemmers::{Algorithm, Stemmer};
use std::collections::HashSet;

lazy_static::lazy_static! {
    static ref STOPWORDS: HashSet<&'static str> = {
        [
            "a", "about", "above", "after", "again", "against", "all", "am", "an", "and",
            "any", "are", "aren't", "as", "at", "be", "because", "been", "before", "being",
            "below", "between", "both", "but", "by", "can't", "cannot", "could", "couldn't",
            "did", "didn't", "do", "does", "doesn't", "doing", "don't", "down", "during",
            "each", "few", "for", "from", "further", "had", "hadn't", "has", "hasn't",
            "have", "haven't", "having", "he", "he'd", "he'll", "he's", "her", "here",
            "here's", "hers", "herself", "him", "himself", "his", "how", "how's", "i",
            "i'd", "i'll", "i'm", "i've", "if", "in", "into", "is", "isn't", "it", "it's",
            "its", "itself", "let's", "me", "more", "most", "mustn't", "my", "myself",
            "no", "nor", "not", "of", "off", "on", "once", "only", "or", "other", "ought",
            "our", "ours", "ourselves", "out", "over", "own", "same", "shan't", "she",
            "she'd", "she'll", "she's", "should", "shouldn't", "so", "some", "such",
            "than", "that", "that's", "the", "their", "theirs", "them", "themselves",
            "then", "there", "there's", "these", "they", "they'd", "they'll", "they're",
            "they've", "this", "those", "through", "to", "too", "under", "until", "up",
            "very", "was", "wasn't", "we", "we'd", "we'll", "we're", "we've", "were",
            "weren't", "what", "what's", "when", "when's", "where", "where's", "which",
            "while", "who", "who's", "whom", "why", "why's", "with", "won't", "would",
            "wouldn't", "you", "you'd", "you'll", "you're", "you've", "your", "yours",
            "yourself", "yourselves",
        ]
        .iter()
        .copied()
        .collect()
    };
}

pub struct Tokenizer {
    stemmer: Stemmer,
}

impl Tokenizer {
    pub fn new() -> Self {
        Self {
            stemmer: Stemmer::create(Algorithm::English),
        }
    }

    /// Tokenize text into words
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.chars()
            .fold(vec![String::new()], |mut tokens, c| {
                if c.is_alphanumeric() {
                    if let Some(last) = tokens.last_mut() {
                        last.push(c);
                    }
                } else if tokens.last().map_or(false, |s| !s.is_empty()) {
                    tokens.push(String::new());
                }
                tokens
            })
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Convert tokens to lowercase
    fn lowercase_filter(&self, tokens: Vec<String>) -> Vec<String> {
        tokens.into_iter().map(|t| t.to_lowercase()).collect()
    }

    /// Remove stopwords
    fn stopword_filter(&self, tokens: Vec<String>) -> Vec<String> {
        tokens
            .into_iter()
            .filter(|t| !STOPWORDS.contains(t.as_str()))
            .collect()
    }

    /// Apply stemming
    fn stemmer_filter(&self, tokens: Vec<String>) -> Vec<String> {
        tokens
            .into_iter()
            .map(|t| self.stemmer.stem(&t).to_string())
            .collect()
    }

    /// Full analysis pipeline
    pub fn analyze(&self, text: &str) -> Vec<String> {
        let tokens = self.tokenize(text);
        let tokens = self.lowercase_filter(tokens);
        let tokens = self.stopword_filter(tokens);
        let tokens = self.stemmer_filter(tokens);
        tokens
    }

    /// Analyze and return unique tokens (for indexing)
    pub fn analyze_unique(&self, text: &str) -> HashSet<String> {
        self.analyze(text).into_iter().collect()
    }

    /// Analyze and count term frequencies
    pub fn analyze_with_frequencies(&self, text: &str) -> std::collections::HashMap<String, usize> {
        let mut frequencies = std::collections::HashMap::new();
        for token in self.analyze(text) {
            *frequencies.entry(token).or_insert(0) += 1;
        }
        frequencies
    }
}

impl Default for Tokenizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize("Hello, World! This is a test.");
        assert_eq!(tokens, vec!["Hello", "World", "This", "is", "a", "test"]);
    }

    #[test]
    fn test_analyze() {
        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.analyze("The quick brown fox jumps");
        // "the" is a stopword, others are stemmed
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
    }
}
