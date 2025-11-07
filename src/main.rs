use clap::{Parser, Subcommand};
use rsfts::{api, Document, SearchEngine, SearchOptions};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "rsfts")]
#[command(about = "Rust Full-Text Search Engine", version, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start HTTP API server
    Serve {
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,

        #[arg(short, long, default_value = "3000")]
        port: u16,

        #[arg(short = 'd', long, default_value = "./data")]
        data_dir: String,
    },

    /// Insert a document (CLI mode)
    Insert {
        #[arg(short, long)]
        id: String,

        #[arg(short, long)]
        title: String,

        #[arg(short, long)]
        content: String,

        #[arg(short = 'u', long)]
        url: Option<String>,

        #[arg(short = 'd', long, default_value = "./data")]
        data_dir: String,
    },

    /// Search for documents (CLI mode)
    Search {
        #[arg(short, long)]
        query: String,

        #[arg(short = 'l', long, default_value = "10")]
        limit: usize,

        #[arg(short = 'r', long, default_value = "true")]
        ranked: bool,

        #[arg(short = 'd', long, default_value = "./data")]
        data_dir: String,
    },

    /// Get document by ID
    Get {
        #[arg(short, long)]
        id: String,

        #[arg(short = 'd', long, default_value = "./data")]
        data_dir: String,
    },

    /// Delete a document
    Delete {
        #[arg(short, long)]
        id: String,

        #[arg(short = 'd', long, default_value = "./data")]
        data_dir: String,
    },

    /// Show index statistics
    Stats {
        #[arg(short = 'd', long, default_value = "./data")]
        data_dir: String,
    },

    /// Import documents from Wikipedia XML dump
    ImportWiki {
        #[arg(short, long)]
        file: String,

        #[arg(short = 'd', long, default_value = "./data")]
        data_dir: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rsfts=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            host,
            port,
            data_dir,
        } => {
            serve(host, port, data_dir).await?;
        }
        Commands::Insert {
            id,
            title,
            content,
            url,
            data_dir,
        } => {
            insert_document(id, title, content, url, data_dir)?;
        }
        Commands::Search {
            query,
            limit,
            ranked,
            data_dir,
        } => {
            search_documents(query, limit, ranked, data_dir)?;
        }
        Commands::Get { id, data_dir } => {
            get_document(id, data_dir)?;
        }
        Commands::Delete { id, data_dir } => {
            delete_document(id, data_dir)?;
        }
        Commands::Stats { data_dir } => {
            show_stats(data_dir)?;
        }
        Commands::ImportWiki { file, data_dir } => {
            import_wiki(file, data_dir)?;
        }
    }

    Ok(())
}

async fn serve(host: String, port: u16, data_dir: String) -> anyhow::Result<()> {
    tracing::info!("Starting search engine with data directory: {}", data_dir);
    let engine = Arc::new(SearchEngine::new(&data_dir)?);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    tracing::info!("Server listening on http://{}", addr);
    tracing::info!("API Documentation:");
    tracing::info!("  GET    /health              - Health check");
    tracing::info!("  POST   /documents           - Insert a document");
    tracing::info!("  POST   /documents/batch     - Batch insert documents");
    tracing::info!("  GET    /documents/:id       - Get a document");
    tracing::info!("  PUT    /documents/:id       - Update a document");
    tracing::info!("  DELETE /documents/:id       - Delete a document");
    tracing::info!("  GET    /search?query=...    - Search documents");
    tracing::info!("  GET    /stats               - Get index statistics");

    let app = api::create_router(engine);

    axum::serve(listener, app).await?;

    Ok(())
}

fn insert_document(
    id: String,
    title: String,
    content: String,
    url: Option<String>,
    data_dir: String,
) -> anyhow::Result<()> {
    let engine = SearchEngine::new(&data_dir)?;

    let mut doc = Document::new(id.clone(), title, content);
    if let Some(url) = url {
        doc = doc.with_url(url);
    }

    engine.upsert_document(doc)?;

    println!("✓ Document '{}' inserted successfully", id);

    Ok(())
}

fn search_documents(query: String, limit: usize, ranked: bool, data_dir: String) -> anyhow::Result<()> {
    let engine = SearchEngine::new(&data_dir)?;

    let options = SearchOptions {
        use_ranking: ranked,
        limit: Some(limit),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = engine.search(&query, &options)?;
    let duration = start.elapsed();

    println!("\n🔍 Search Results for: \"{}\"", query);
    println!("Found {} documents in {:?}", result.total, duration);
    println!();

    for (i, doc) in result.documents.iter().enumerate() {
        if let Some(scores) = &result.scores {
            println!("{}. [Score: {:.4}] {}", i + 1, scores[i], doc.title);
        } else {
            println!("{}. {}", i + 1, doc.title);
        }
        println!("   ID: {}", doc.id);
        if let Some(url) = &doc.url {
            println!("   URL: {}", url);
        }
        println!("   Content: {}...", &doc.content[..doc.content.len().min(100)]);
        println!();
    }

    Ok(())
}

fn get_document(id: String, data_dir: String) -> anyhow::Result<()> {
    let engine = SearchEngine::new(&data_dir)?;

    if let Some(doc) = engine.get_document(&id)? {
        println!("\n📄 Document");
        println!("ID:      {}", doc.id);
        println!("Title:   {}", doc.title);
        if let Some(url) = &doc.url {
            println!("URL:     {}", url);
        }
        println!("Content: {}", doc.content);
        println!();
    } else {
        println!("❌ Document '{}' not found", id);
    }

    Ok(())
}

fn delete_document(id: String, data_dir: String) -> anyhow::Result<()> {
    let engine = SearchEngine::new(&data_dir)?;
    engine.delete_document(&id)?;
    println!("✓ Document '{}' deleted successfully", id);
    Ok(())
}

fn show_stats(data_dir: String) -> anyhow::Result<()> {
    let engine = SearchEngine::new(&data_dir)?;
    let stats = engine.stats()?;

    println!("\n📊 Index Statistics");
    println!("Total Documents:       {}", stats.total_documents);
    println!("Total Unique Tokens:   {}", stats.total_tokens);
    println!("Avg Docs per Token:    {:.2}", stats.avg_docs_per_token);
    println!();

    Ok(())
}

fn import_wiki(file: String, data_dir: String) -> anyhow::Result<()> {
    use flate2::read::GzDecoder;
    use quick_xml::de::from_reader;
    use serde::Deserialize;
    use std::fs::File;
    use std::io::BufReader;

    #[derive(Debug, Deserialize)]
    struct WikiDoc {
        #[serde(default)]
        title: String,
        #[serde(default)]
        url: String,
        #[serde(rename = "abstract", default)]
        text: String,
    }

    #[derive(Debug, Deserialize)]
    struct Feed {
        #[serde(rename = "doc", default)]
        documents: Vec<WikiDoc>,
    }

    println!("Loading Wikipedia dump from: {}", file);

    let f = File::open(&file)?;
    let decoder = GzDecoder::new(f);
    let reader = BufReader::new(decoder);

    let mut feed: Feed = from_reader(reader)?;

    println!("Loaded {} documents", feed.documents.len());
    println!("Indexing documents...");

    let engine = SearchEngine::new(&data_dir)?;

    let docs: Vec<Document> = feed
        .documents
        .drain(..)
        .enumerate()
        .map(|(i, d)| {
            Document::new(i.to_string(), d.title, d.text).with_url(d.url)
        })
        .collect();

    let total = docs.len();
    engine.batch_insert(docs)?;

    println!("✓ Successfully imported {} documents", total);

    Ok(())
}
