use clap::Parser;
use flate2::read::GzDecoder;
use quick_xml::de::from_reader;
use rust_stemmers::{Algorithm, Stemmer};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;
use std::time::Instant;

// CLI Arguments
#[derive(Parser, Debug)]
#[command(author, version, about = "Simple Full-Text Search engine in Rust", long_about = None)]
struct Args {
    #[arg(short, long, default_value = "enwiki-latest-abstract1.xml.gz")]
    path: String,

    #[arg(short, long, default_value = "Small wild cat")]
    query: String,
}

// Document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Document {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(rename = "abstract", default)]
    text: String,
    #[serde(skip_deserializing)]
    id: usize,
}

// Wrapper for XML deserialization
#[derive(Debug, Deserialize)]
struct Feed {
    #[serde(rename = "doc", default)]
    documents: Vec<Document>,
}

// Inverted index type
type Index = HashMap<String, Vec<usize>>;

// Document loading
fn load_documents(path: &str) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let decoder = GzDecoder::new(file);
    let reader = BufReader::new(decoder);

    let mut feed: Feed = from_reader(reader)?;

    // Assign IDs to documents
    for (i, doc) in feed.documents.iter_mut().enumerate() {
        doc.id = i;
    }

    Ok(feed.documents)
}

// Save documents to JSON
fn save_docs_as_json(docs: &[Document], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(docs)?;
    let mut file = File::create(filename)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

// Load documents from JSON
fn load_docs_from_json(filename: &str) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let docs = serde_json::from_reader(reader)?;
    Ok(docs)
}

// Save index to JSON
fn save_index_as_json(index: &Index, filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string_pretty(index)?;
    let mut file = File::create(filename)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

// Load index from JSON
fn load_index_from_json(filename: &str) -> Result<Index, Box<dyn std::error::Error>> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let index = serde_json::from_reader(reader)?;
    Ok(index)
}

// Tokenizer
fn tokenize(text: &str) -> Vec<String> {
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

// Lowercase filter
fn lowercase_filter(tokens: Vec<String>) -> Vec<String> {
    tokens.into_iter().map(|t| t.to_lowercase()).collect()
}

// Stopword filter
fn stopword_filter(tokens: Vec<String>) -> Vec<String> {
    let stopwords: HashSet<&str> = ["a", "and", "be", "have", "i", "in", "of", "that", "the", "to"]
        .iter()
        .copied()
        .collect();

    tokens
        .into_iter()
        .filter(|t| !stopwords.contains(t.as_str()))
        .collect()
}

// Stemmer filter
fn stemmer_filter(tokens: Vec<String>) -> Vec<String> {
    let stemmer = Stemmer::create(Algorithm::English);
    tokens
        .into_iter()
        .map(|t| stemmer.stem(&t).to_string())
        .collect()
}

// Analyze text (full pipeline)
fn analyze(text: &str) -> Vec<String> {
    let tokens = tokenize(text);
    let tokens = lowercase_filter(tokens);
    let tokens = stopword_filter(tokens);
    let tokens = stemmer_filter(tokens);
    tokens
}

// Add documents to index
fn add_to_index(index: &mut Index, docs: &[Document]) {
    for doc in docs {
        for token in analyze(&doc.text) {
            let ids = index.entry(token).or_insert_with(Vec::new);
            // Don't add same ID twice
            if ids.last() != Some(&doc.id) {
                ids.push(doc.id);
            }
        }
    }
}

// Intersection of two sorted arrays
fn intersection(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut result = Vec::new();
    let mut i = 0;
    let mut j = 0;

    while i < a.len() && j < b.len() {
        if a[i] < b[j] {
            i += 1;
        } else if a[i] > b[j] {
            j += 1;
        } else {
            result.push(a[i]);
            i += 1;
            j += 1;
        }
    }

    result
}

// Search in index
fn search(index: &Index, text: &str) -> Vec<usize> {
    let tokens = analyze(text);
    let mut result: Option<Vec<usize>> = None;

    for token in tokens {
        if let Some(ids) = index.get(&token) {
            result = Some(match result {
                None => ids.clone(),
                Some(r) => intersection(&r, ids),
            });
        } else {
            // Token doesn't exist
            return Vec::new();
        }
    }

    result.unwrap_or_default()
}

// Perform search and display results
fn do_search(index: &Index, query: &str, docs: &[Document]) {
    let start = Instant::now();
    let matched_ids = search(index, query);
    let duration = start.elapsed();

    println!("Search found {} documents in {:?}", matched_ids.len(), duration);
    println!();

    for id in matched_ids {
        if let Some(doc) = docs.get(id) {
            println!("{}\t{}", id, doc.text);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    println!("Starting rsfts (Rust Full-Text Search)");

    // Load or deserialize documents
    let docs = if Path::new("doc.json").exists() {
        println!("Loading documents from cache...");
        load_docs_from_json("doc.json")?
    } else {
        let start = Instant::now();
        let docs = load_documents(&args.path)?;
        let duration = start.elapsed();
        println!("Loaded {} documents in {:?}", docs.len(), duration);

        save_docs_as_json(&docs, "doc.json")?;
        docs
    };

    // Build or load index
    let index = if Path::new("index.json").exists() {
        println!("Loading index from cache...");
        load_index_from_json("index.json")?
    } else {
        let start = Instant::now();
        let mut index = HashMap::new();
        add_to_index(&mut index, &docs);
        let duration = start.elapsed();
        println!("Indexed {} documents in {:?}", docs.len(), duration);

        save_index_as_json(&index, "index.json")?;
        index
    };

    // Perform search
    println!();
    println!("Searching for: \"{}\"", args.query);
    println!();
    do_search(&index, &args.query, &docs);

    Ok(())
}
