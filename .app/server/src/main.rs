use axum::{
    Router,
    body::Body,
    extract::Path,
    http::{Request, StatusCode},
    response::{IntoResponse, Json},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{net::SocketAddr, path::PathBuf, sync::OnceLock};
use tower_http::services::ServeDir;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

fn root() -> &'static PathBuf {
    ROOT.get().expect("ROOT not set")
}

fn data_path() -> PathBuf {
    root().join(".data/annotations.json")
}

// ── Catalog ──

#[derive(Serialize, Deserialize)]
struct BookEntry {
    author: String,
    year: u32,
    title: String,
    category: String,
    path: String,
    ext: String,
    file: String,
}

fn parse_filename(filename: &str, rel_path: &str, category: &str) -> BookEntry {
    let ext = if filename.ends_with(".epub") { "epub" } else { "pdf" };
    let stem = filename.trim_end_matches(".pdf").trim_end_matches(".epub");

    // Try to parse: Author.Year.Title.Category
    let parts: Vec<&str> = stem.splitn(4, '.').collect();
    if parts.len() >= 3 {
        if let Ok(year) = parts[1].parse::<u32>() {
            let author = parts[0]
                .replace("AndOthers", " & Others")
                .replace("And", " & ");
            let title = camel_to_spaces(parts[2]);
            return BookEntry {
                author,
                year,
                title,
                category: category.to_string(),
                path: rel_path.to_string(),
                ext: ext.to_string(),
                file: filename.to_string(),
            };
        }
    }

    // Fallback: use filename as title
    BookEntry {
        author: String::new(),
        year: 0,
        title: stem.replace('.', " ").replace('_', " "),
        category: category.to_string(),
        path: rel_path.to_string(),
        ext: ext.to_string(),
        file: filename.to_string(),
    }
}

fn camel_to_spaces(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 10);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            let prev = s.chars().nth(i - 1).unwrap_or(' ');
            if prev.is_lowercase() {
                result.push(' ');
            }
        }
        result.push(c);
    }
    result
}

fn scan_library(root: &std::path::Path) -> Vec<BookEntry> {
    let mut books = Vec::new();
    scan_dir(root, root, &mut books);
    books.sort_by(|a, b| a.file.cmp(&b.file));
    books
}

fn scan_dir(root: &std::path::Path, dir: &std::path::Path, books: &mut Vec<BookEntry>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden dirs/files
        if name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            scan_dir(root, &path, books);
        } else if name.ends_with(".pdf") || name.ends_with(".epub") {
            let parent = path.parent().unwrap_or(root);
            let rel_path = parent
                .strip_prefix(root)
                .unwrap_or(parent)
                .to_string_lossy()
                .to_string();
            let category = parent
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            books.push(parse_filename(&name, &rel_path, &category));
        }
    }
}

async fn get_catalog() -> impl IntoResponse {
    let books = scan_library(root());
    Json(books)
}

// ── Data ──

async fn get_data() -> impl IntoResponse {
    match tokio::fs::read_to_string(data_path()).await {
        Ok(contents) => {
            let json: Value =
                serde_json::from_str(&contents).unwrap_or(Value::Object(Default::default()));
            Json(json).into_response()
        }
        Err(_) => Json(serde_json::json!({})).into_response(),
    }
}

async fn post_data(Json(body): Json<Value>) -> impl IntoResponse {
    // Ensure .data/ dir exists
    let _ = tokio::fs::create_dir_all(root().join(".data")).await;
    let pretty = serde_json::to_string_pretty(&body).unwrap_or_default();
    match tokio::fs::write(data_path(), pretty).await {
        Ok(_) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

// ── Open file ──

async fn open_file(Path(path): Path<String>) -> impl IntoResponse {
    let full = root().join(&path);
    if full.is_file() {
        let _ = open::that_detached(&full);
        (StatusCode::OK, Json(serde_json::json!({"ok": true})))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "file not found"})),
        )
    }
}

// ── Fallback ──

async fn index_fallback(req: Request<Body>) -> impl IntoResponse {
    let path = req.uri().path();
    if path == "/" {
        axum::response::Redirect::temporary("/.app/library.html").into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

#[tokio::main]
async fn main() {
    let library_root = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().expect("no cwd"));

    ROOT.set(library_root.clone()).expect("ROOT already set");

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8384);

    let serve_dir = ServeDir::new(&library_root).fallback(axum::routing::get(index_fallback));

    let app = Router::new()
        .route("/api/catalog", get(get_catalog))
        .route("/api/data", get(get_data).post(post_data))
        .route("/open/{*path}", get(open_file))
        .fallback_service(serve_dir);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Library at http://localhost:{port}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
