use rsfts::{Document, SearchEngine, SearchMode, SearchOptions};

fn main() -> anyhow::Result<()> {
    println!("=== RSFTS Basic Usage Example ===\n");

    // Create a new search engine (in-memory for this example)
    let engine = SearchEngine::in_memory()?;

    // Insert some documents
    println!("Inserting documents...");

    let doc1 = Document::new(
        "1".to_string(),
        "Rust Programming Language".to_string(),
        "Rust is a systems programming language that runs blazingly fast, prevents segfaults, and guarantees thread safety.".to_string(),
    ).with_url("https://www.rust-lang.org".to_string());

    let doc2 = Document::new(
        "2".to_string(),
        "Go Programming Language".to_string(),
        "Go is an open source programming language that makes it easy to build simple, reliable, and efficient software.".to_string(),
    ).with_url("https://golang.org".to_string());

    let doc3 = Document::new(
        "3".to_string(),
        "Python Programming".to_string(),
        "Python is a programming language that lets you work quickly and integrate systems more effectively.".to_string(),
    ).with_url("https://www.python.org".to_string());

    engine.upsert_document(doc1)?;
    engine.upsert_document(doc2)?;
    engine.upsert_document(doc3)?;

    println!("✓ Inserted 3 documents\n");

    // Example 1: Basic search with ranking
    println!("--- Example 1: Search for 'programming language' ---");
    let results = engine.search("programming language", &SearchOptions::default())?;

    println!("Found {} documents", results.total);
    for (i, doc) in results.documents.iter().enumerate() {
        if let Some(scores) = &results.scores {
            println!("\n{}. [Score: {:.4}] {}", i + 1, scores[i], doc.title);
        } else {
            println!("\n{}. {}", i + 1, doc.title);
        }
        println!("   URL: {}", doc.url.as_ref().unwrap_or(&"N/A".to_string()));
        println!("   Content: {}...", &doc.content[..doc.content.len().min(80)]);
    }

    // Example 2: Search without ranking
    println!("\n\n--- Example 2: Search without ranking ---");
    let options = SearchOptions {
        use_ranking: false,
        ..Default::default()
    };
    let results = engine.search("programming", &options)?;
    println!("Found {} documents (unranked)", results.total);

    // Example 3: OR search (any term matches)
    println!("\n\n--- Example 3: OR search for 'rust python' ---");
    let options = SearchOptions {
        mode: SearchMode::Or,
        ..Default::default()
    };
    let results = engine.search("rust python", &options)?;
    println!("Found {} documents matching ANY term", results.total);
    for (i, doc) in results.documents.iter().enumerate() {
        println!("  {}. {}", i + 1, doc.title);
    }

    // Example 4: Pagination
    println!("\n\n--- Example 4: Pagination (2 results per page) ---");
    let options = SearchOptions {
        limit: Some(2),
        offset: 0,
        ..Default::default()
    };
    let results = engine.search("programming", &options)?;
    println!("Page 1 (showing 2 of {} total):", results.total);
    for (i, doc) in results.documents.iter().enumerate() {
        println!("  {}. {}", i + 1, doc.title);
    }

    // Example 5: Delete and update
    println!("\n\n--- Example 5: Delete a document ---");
    engine.delete_document("2")?;
    println!("✓ Deleted document '2' (Go Programming)");

    let results = engine.search("programming language", &SearchOptions::default())?;
    println!("After deletion, found {} documents", results.total);

    // Example 6: Update a document
    println!("\n\n--- Example 6: Update a document ---");
    let updated_doc = Document::new(
        "1".to_string(),
        "Rust: A Modern Systems Programming Language".to_string(),
        "Rust is a modern systems programming language focused on safety, speed, and concurrency.".to_string(),
    );
    engine.upsert_document(updated_doc)?;
    println!("✓ Updated document '1'");

    if let Some(doc) = engine.get_document("1")? {
        println!("New title: {}", doc.title);
    }

    // Example 7: Statistics
    println!("\n\n--- Example 7: Index Statistics ---");
    let stats = engine.stats()?;
    println!("Total documents: {}", stats.total_documents);
    println!("Total unique tokens: {}", stats.total_tokens);
    println!("Average docs per token: {:.2}", stats.avg_docs_per_token);

    println!("\n=== Example Complete ===");

    Ok(())
}
