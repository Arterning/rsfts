use crate::document::Document;
use crate::engine::{SearchEngine, SearchMode, SearchOptions};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ========== Request/Response Types ==========

#[derive(Debug, Deserialize)]
pub struct InsertDocumentRequest {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BatchInsertRequest {
    pub documents: Vec<InsertDocumentRequest>,
}

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default)]
    pub mode: Option<String>, // "and" or "or"
    #[serde(default)]
    pub ranked: Option<bool>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub documents: Vec<DocumentResponse>,
    pub total: usize,
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scores: Option<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct DocumentResponse {
    pub id: String,
    pub title: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl From<Document> for DocumentResponse {
    fn from(doc: Document) -> Self {
        Self {
            id: doc.id,
            title: doc.title,
            content: doc.content,
            url: doc.url,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub total_documents: usize,
    pub total_tokens: usize,
    pub avg_docs_per_token: f64,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    fn error_msg(message: String) -> Self {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message),
        }
    }

    fn error(message: String) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

// ========== Error Handling ==========

struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let message = format!("{:#}", self.0);
        tracing::error!("API error: {}", message);

        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(message)),
        )
            .into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

// ========== Handlers ==========

async fn health_check() -> impl IntoResponse {
    Json(ApiResponse::success("OK"))
}

async fn insert_document(
    State(engine): State<Arc<SearchEngine>>,
    Json(req): Json<InsertDocumentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let mut doc = Document::new(req.id, req.title, req.content);
    if let Some(url) = req.url {
        doc = doc.with_url(url);
    }

    engine.upsert_document(doc)?;

    Ok(Json(ApiResponse::success("Document inserted successfully")))
}

async fn batch_insert(
    State(engine): State<Arc<SearchEngine>>,
    Json(req): Json<BatchInsertRequest>,
) -> Result<impl IntoResponse, AppError> {
    let docs: Vec<Document> = req
        .documents
        .into_iter()
        .map(|d| {
            let mut doc = Document::new(d.id, d.title, d.content);
            if let Some(url) = d.url {
                doc = doc.with_url(url);
            }
            doc
        })
        .collect();

    engine.batch_insert(docs)?;

    Ok(Json(ApiResponse::success("Documents inserted successfully")))
}

async fn get_document(
    State(engine): State<Arc<SearchEngine>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    if let Some(doc) = engine.get_document(&id)? {
        Ok(Json(ApiResponse::success(DocumentResponse::from(doc))))
    } else {
        Ok(Json(ApiResponse::error_msg(format!(
            "Document with id '{}' not found",
            id
        ))))
    }
}

async fn update_document(
    State(engine): State<Arc<SearchEngine>>,
    Path(id): Path<String>,
    Json(req): Json<InsertDocumentRequest>,
) -> Result<impl IntoResponse, AppError> {
    let mut doc = Document::new(id, req.title, req.content);
    if let Some(url) = req.url {
        doc = doc.with_url(url);
    }

    engine.upsert_document(doc)?;

    Ok(Json(ApiResponse::success("Document updated successfully")))
}

async fn delete_document(
    State(engine): State<Arc<SearchEngine>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    engine.delete_document(&id)?;
    Ok(Json(ApiResponse::success("Document deleted successfully")))
}

async fn search_documents(
    State(engine): State<Arc<SearchEngine>>,
    Query(req): Query<SearchRequest>,
) -> Result<impl IntoResponse, AppError> {
    let mode = match req.mode.as_deref() {
        Some("or") => SearchMode::Or,
        _ => SearchMode::And,
    };

    let options = SearchOptions {
        mode,
        use_ranking: req.ranked.unwrap_or(true),
        limit: req.limit.or(Some(10)),
        offset: req.offset.unwrap_or(0),
    };

    let result = engine.search(&req.query, &options)?;

    let response = SearchResponse {
        documents: result.documents.into_iter().map(DocumentResponse::from).collect(),
        total: result.total,
        query: req.query,
        scores: result.scores,
    };

    Ok(Json(ApiResponse::success(response)))
}

async fn get_stats(State(engine): State<Arc<SearchEngine>>) -> Result<impl IntoResponse, AppError> {
    let stats = engine.stats()?;

    let response = StatsResponse {
        total_documents: stats.total_documents,
        total_tokens: stats.total_tokens,
        avg_docs_per_token: stats.avg_docs_per_token,
    };

    Ok(Json(ApiResponse::success(response)))
}

// ========== Router ==========

pub fn create_router(engine: Arc<SearchEngine>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/documents", post(insert_document))
        .route("/documents/batch", post(batch_insert))
        .route("/documents/:id", get(get_document))
        .route("/documents/:id", put(update_document))
        .route("/documents/:id", delete(delete_document))
        .route("/search", get(search_documents))
        .route("/stats", get(get_stats))
        .with_state(engine)
}
