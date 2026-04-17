# Cognix: Full Rust Migration Plan

## Architecture: Modular Monolith + Screaming Architecture + DDD

Single deployable (Tauri .dmg), but internally organized as independent domain modules with clear boundaries, event-driven communication, and full observability.

```
cognix/
│
├── src-tauri/                          # Backend (Rust)
│   ├── src/
│   │   ├── main.rs                     # Tauri entry, wires everything
│   │   │
│   │   ├── core/                       # Shared infrastructure
│   │   │   ├── mod.rs
│   │   │   ├── types.rs                # Domain primitives (BookId, ConceptId, PageRef, etc.)
│   │   │   ├── errors.rs               # DomainError base, Result<T> alias
│   │   │   ├── events.rs              # DomainEvent trait, EventBus
│   │   │   └── security.rs            # RBAC (for future collab: owner/editor/viewer)
│   │   │
│   │   ├── catalog/                    # Domain: Book catalog & filesystem
│   │   │   ├── mod.rs
│   │   │   ├── domain.rs              # Book, Category, FileMetadata
│   │   │   ├── repository.rs          # BookRepository trait + SQLite impl
│   │   │   ├── service.rs             # CatalogService (scan, index, search)
│   │   │   ├── scanner.rs             # Filesystem walker, filename parser
│   │   │   ├── commands.rs            # Tauri commands (#[tauri::command])
│   │   │   └── events.rs             # BookAdded, BookRemoved, CatalogScanned
│   │   │
│   │   ├── reader/                     # Domain: Document rendering
│   │   │   ├── mod.rs
│   │   │   ├── domain.rs              # PageContent, TextLayer, Viewport
│   │   │   ├── pdf.rs                 # PDF rendering via pdfium-render
│   │   │   ├── epub.rs                # EPUB parsing via epub-rs
│   │   │   ├── commands.rs            # Tauri commands (get_page, get_text_layer)
│   │   │   └── events.rs             # PageViewed, DocumentOpened
│   │   │
│   │   ├── annotations/               # Domain: Concepts, markers, highlights
│   │   │   ├── mod.rs
│   │   │   ├── domain.rs              # Concept, Marker, Highlight, Tag
│   │   │   ├── repository.rs          # AnnotationRepository trait + SQLite impl
│   │   │   ├── service.rs             # AnnotationService (CRUD, autocomplete, search)
│   │   │   ├── commands.rs            # Tauri commands
│   │   │   └── events.rs             # ConceptCreated, MarkerAdded, HighlightCreated
│   │   │
│   │   ├── knowledge_graph/            # Domain: Concept-book graph
│   │   │   ├── mod.rs
│   │   │   ├── domain.rs              # GraphNode, GraphEdge, GraphLayout
│   │   │   ├── service.rs             # GraphService (build bipartite graph, compute layout)
│   │   │   ├── commands.rs            # Tauri commands (get_graph_data)
│   │   │   └── events.rs             # GraphRecomputed
│   │   │
│   │   ├── search/                     # Domain: Full-text search
│   │   │   ├── mod.rs
│   │   │   ├── domain.rs              # SearchResult, SearchQuery
│   │   │   ├── indexer.rs             # FTS5 index management
│   │   │   ├── service.rs             # SearchService (query annotations + PDF text)
│   │   │   └── commands.rs            # Tauri commands
│   │   │
│   │   ├── sync/                       # Domain: Multi-device sync (future)
│   │   │   ├── mod.rs
│   │   │   ├── domain.rs              # SyncState, ChangeLog, Conflict
│   │   │   ├── engine.rs              # Offline-first sync engine
│   │   │   └── commands.rs
│   │   │
│   │   ├── storage/                    # Infrastructure: Database
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs          # SQLite connection pool
│   │   │   ├── migrations.rs          # Schema migrations runner
│   │   │   └── migrations/
│   │   │       ├── 001_initial.sql
│   │   │       ├── 002_fts_indexes.sql
│   │   │       └── 003_sync_changelog.sql
│   │   │
│   │   └── observability/              # Infrastructure: Logging, tracing, metrics
│   │       ├── mod.rs
│   │       ├── logging.rs             # Structured logging (tracing crate)
│   │       ├── tracing.rs             # Request tracing, spans for commands
│   │       ├── metrics.rs             # App metrics (books count, annotations count, read time)
│   │       └── health.rs              # Health check (db connected, library readable)
│   │
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── build.rs
│
├── src/                                # Frontend (Leptos/WASM) — 100% Rust
│   ├── main.rs                         # WASM entry
│   ├── app.rs                          # Root component, router, global state
│   │
│   ├── catalog/                        # UI: Book browsing
│   │   ├── mod.rs
│   │   ├── cards_view.rs              # Cards grid
│   │   ├── card.rs                    # Single book card
│   │   ├── filters.rs                 # Search, category, year, format, tag filters
│   │   └── sort.rs                    # Sort toggle
│   │
│   ├── reader/                         # UI: Document reader
│   │   ├── mod.rs
│   │   ├── reader_view.rs             # Split layout container
│   │   ├── pdf_viewer.rs              # PDF canvas rendering (pdfium via Tauri)
│   │   ├── epub_viewer.rs             # EPUB HTML rendering
│   │   ├── toolbar.rs                 # Reader top bar (back, title, toggle panel)
│   │   └── highlight_layer.rs         # SVG overlay for highlights + selection
│   │
│   ├── annotations/                    # UI: Concepts, markers, tags
│   │   ├── mod.rs
│   │   ├── modal.rs                   # Book detail modal
│   │   ├── panel.rs                   # Side panel in reader
│   │   ├── concept_list.rs            # List of concepts with markers
│   │   ├── marker_item.rs             # Single marker (page + note + highlights)
│   │   ├── tag_input.rs               # Tag pill input
│   │   └── autocomplete.rs            # Concept name autocomplete
│   │
│   ├── knowledge_graph/                # UI: Graph visualization
│   │   ├── mod.rs
│   │   ├── graph_view.rs              # SVG force-directed graph (pure Rust)
│   │   ├── graph_node.rs              # Book node (rect) + concept node (circle)
│   │   ├── graph_pelito.rs            # Marker satellite dots
│   │   ├── graph_popup.rs             # Click popup on pelito
│   │   └── force_simulation.rs        # Force-directed layout algorithm
│   │
│   ├── notes/                          # UI: Global annotations view
│   │   ├── mod.rs
│   │   ├── notes_view.rs              # All concepts with accordion
│   │   └── notes_search.rs            # Search across all annotations
│   │
│   └── shared/                         # Shared UI components
│       ├── mod.rs
│       ├── types.rs                   # Frontend models (mirrors backend domain)
│       ├── api.rs                     # Tauri invoke wrappers
│       ├── icons.rs                   # SVG icons as components
│       └── theme.rs                   # CSS variables, dark theme
│
├── assets/                             # Static files
│   └── styles/
│       └── app.css                    # Global styles
│
├── tests/                              # Testing
│   ├── unit/
│   │   ├── catalog_test.rs            # Filename parser, scanner
│   │   ├── annotations_test.rs        # CRUD, autocomplete logic
│   │   ├── graph_test.rs              # Graph building, force layout
│   │   └── search_test.rs             # FTS queries
│   ├── integration/
│   │   ├── db_test.rs                 # SQLite migrations, repository impls
│   │   ├── catalog_service_test.rs    # Full scan → index flow
│   │   └── annotation_service_test.rs # Create concept → add marker → search
│   └── e2e/
│       └── app_test.rs                # Tauri WebDriver tests
│
├── tools/                              # Architecture enforcement
│   ├── boundary_checker.rs            # Validates module dependencies
│   └── arch_rules.toml                # Allowed/forbidden imports per module
│
├── Cargo.toml                          # Workspace
├── Trunk.toml                          # Leptos/WASM build config
├── README.md
├── ROADMAP.md
├── MIGRATION.md
└── LICENSE
```

---

## Phase 1: Core Infrastructure

```
- [ ] Create Cargo workspace (Cargo.toml) with src-tauri + src crates
- [ ] Scaffold Tauri 2.0 project with Leptos template
- [ ] core/types.rs — domain primitives:
      BookId(i64), ConceptId(i64), MarkerId(i64), HighlightId(i64)
      PageRef(String), FilePath(PathBuf), FileHash(String)
- [ ] core/errors.rs — DomainError enum:
      NotFound, AlreadyExists, InvalidInput, StorageError, RenderError
      impl From<rusqlite::Error>, impl IntoResponse for Tauri
- [ ] core/events.rs — event system:
      trait DomainEvent: Send + Sync + 'static
      struct EventBus { handlers: HashMap<TypeId, Vec<Box<dyn Fn>>> }
      impl EventBus { fn publish<E: DomainEvent>(&self, event: E) }
      impl EventBus { fn subscribe<E: DomainEvent>(&self, handler: impl Fn(E)) }
- [ ] core/security.rs — RBAC:
      enum Role { Owner, Editor, Viewer }
      struct Permission { resource: String, action: Action }
      fn check_permission(role: Role, action: Action) -> Result<()>
```

## Phase 2: Storage Infrastructure

```
- [ ] storage/connection.rs — SQLite pool:
      fn init_db(path: &Path) -> Result<Connection>
      fn run_migrations(conn: &Connection) -> Result<()>
- [ ] storage/migrations/001_initial.sql — full schema (books, concepts,
      book_concepts, markers, highlights, tags)
- [ ] storage/migrations/002_fts_indexes.sql — FTS5 virtual tables
- [ ] Data migration: read annotations.json → insert into SQLite on first run
```

## Phase 3: Observability

```
- [ ] observability/logging.rs — structured logging:
      Use `tracing` crate with `tracing-subscriber`
      JSON format for structured logs
      Log levels: TRACE for queries, DEBUG for service calls,
      INFO for commands, WARN for recoverable errors, ERROR for failures
- [ ] observability/tracing.rs — request tracing:
      #[instrument] on every Tauri command
      Span hierarchy: command → service → repository → query
      Duration tracking per span
- [ ] observability/metrics.rs — app metrics:
      struct AppMetrics { books_count, concepts_count, markers_count,
      highlights_count, avg_concepts_per_book, most_connected_concept }
      fn collect_metrics(db: &Connection) -> AppMetrics
      Tauri command to expose metrics to frontend
- [ ] observability/health.rs — health check:
      fn health_check() -> HealthStatus
      Checks: db connection, library path readable, disk space
```

## Phase 4: Catalog Module

```
- [ ] catalog/domain.rs:
      struct Book { id, file_name, author, year, title, category, path, ext, file_hash }
      struct Category { name: String, parent: Option<String> }
      struct FileMetadata { size: u64, modified: SystemTime, hash: String }
- [ ] catalog/scanner.rs:
      fn scan_library(root: &Path) -> Vec<ScannedFile>
      fn parse_filename(name: &str) -> ParsedMetadata  (port from current Rust)
      fn compute_hash(path: &Path) -> String
- [ ] catalog/repository.rs:
      trait BookRepository { fn upsert, fn find_by_id, fn find_all, fn search, fn delete }
      struct SqliteBookRepository { conn: Connection }
- [ ] catalog/service.rs:
      struct CatalogService { repo: Box<dyn BookRepository>, bus: EventBus }
      fn full_scan(&self, root: &Path) → scan, diff with DB, upsert new, remove deleted
      fn search_books(&self, query: &str, filters: Filters) -> Vec<Book>
- [ ] catalog/commands.rs:
      #[tauri::command] fn get_catalog() -> Vec<Book>
      #[tauri::command] fn rescan_library()
      #[tauri::command] fn get_book(id: BookId) -> Book
- [ ] catalog/events.rs:
      struct BookAdded { book: Book }
      struct BookRemoved { book_id: BookId }
      struct CatalogScanned { added: usize, removed: usize, total: usize }
```

## Phase 5: Annotations Module

```
- [ ] annotations/domain.rs:
      struct Concept { id, name, created_at }
      struct BookConcept { id, book_id, concept_id }
      struct Marker { id, book_concept_id, page, note, created_at, updated_at }
      struct Highlight { id, marker_id, page, text, rects: Vec<Rect>, created_at }
      struct Tag { id, book_id, name }
      struct Rect { x: f64, y: f64, w: f64, h: f64 }
- [ ] annotations/repository.rs:
      trait AnnotationRepository {
        fn upsert_concept, fn add_marker, fn update_marker, fn delete_marker,
        fn add_highlight, fn add_tag, fn remove_tag,
        fn get_book_annotations, fn get_all_concepts,
        fn autocomplete_concept(prefix: &str) -> Vec<String>
      }
      struct SqliteAnnotationRepository
- [ ] annotations/service.rs:
      struct AnnotationService { repo, bus }
      fn get_book_data(book_id) → full concept tree with markers + highlights
      fn add_concept_with_marker(book_id, concept_name, page, note) → atomic
      fn add_highlight_to_marker(marker_id, page, text, rects)
      fn search_annotations(query: &str) → FTS5 search
- [ ] annotations/commands.rs:
      All Tauri commands for CRUD operations
- [ ] annotations/events.rs:
      ConceptCreated, MarkerAdded, MarkerUpdated, HighlightCreated, TagAdded
```

## Phase 6: Reader Module

```
- [ ] reader/domain.rs:
      struct PageContent { page_num: u32, width: f64, height: f64 }
      struct TextItem { text: String, x: f64, y: f64, font_size: f64 }
      struct TextLayer { items: Vec<TextItem> }
- [ ] reader/pdf.rs:
      Use pdfium-render crate
      fn open_pdf(path: &Path) -> PdfDocument
      fn render_page(doc, page_num, scale) -> Vec<u8>  (PNG bytes)
      fn get_text_layer(doc, page_num, scale) -> TextLayer
      fn get_page_count(doc) -> u32
- [ ] reader/epub.rs:
      Use epub-rs crate
      fn open_epub(path: &Path) -> EpubDocument
      fn get_chapter_html(doc, chapter_idx) -> String
      fn get_toc(doc) -> Vec<TocEntry>
- [ ] reader/commands.rs:
      #[tauri::command] fn open_document(path: String) -> DocumentInfo
      #[tauri::command] fn get_pdf_page(doc_id, page, scale) -> Base64Image
      #[tauri::command] fn get_pdf_text_layer(doc_id, page, scale) -> TextLayer
      #[tauri::command] fn get_epub_chapter(doc_id, chapter) -> String
      #[tauri::command] fn get_page_count(doc_id) -> u32
- [ ] reader/events.rs:
      DocumentOpened { book_id, doc_type }, PageViewed { book_id, page }
```

## Phase 7: Knowledge Graph Module

```
- [ ] knowledge_graph/domain.rs:
      enum NodeType { Book(BookId), Concept(ConceptId), Marker(MarkerId) }
      struct GraphNode { id, node_type, x, y, r, label, color }
      struct GraphEdge { source, target }
      struct GraphData { nodes: Vec<GraphNode>, edges: Vec<GraphEdge> }
- [ ] knowledge_graph/service.rs:
      fn build_graph(books, annotations) -> GraphData
      fn force_directed_layout(data: &mut GraphData, iterations: u32)
        Barnes-Hut approximation for O(n log n) performance
      fn filter_graph(data, concept_filter, book_filter) -> GraphData
- [ ] knowledge_graph/commands.rs:
      #[tauri::command] fn get_graph_data(filters) -> GraphData
```

## Phase 8: Search Module

```
- [ ] search/domain.rs:
      struct SearchQuery { text: String, scope: SearchScope }
      enum SearchScope { All, Books, Concepts, Markers, Highlights }
      struct SearchResult { result_type, title, snippet, book_id, page }
- [ ] search/indexer.rs:
      fn rebuild_index(conn: &Connection)
      fn index_marker(conn, marker: &Marker)
      fn index_highlight(conn, highlight: &Highlight)
- [ ] search/service.rs:
      fn search(query: &SearchQuery) -> Vec<SearchResult>
      Uses FTS5 with ranking
- [ ] search/commands.rs:
      #[tauri::command] fn search(query: String, scope: String) -> Vec<SearchResult>
```

## Phase 9: Leptos Frontend

```
- [ ] shared/types.rs — mirror backend types with Serialize/Deserialize
- [ ] shared/api.rs — Tauri invoke wrappers:
      async fn invoke_get_catalog() -> Vec<Book>
      async fn invoke_get_book_data(id: i64) -> BookData
      etc.
- [ ] shared/theme.rs — CSS variables as constants
- [ ] app.rs — root component with signals:
      view_mode: Signal<ViewMode>,
      books: Resource<Vec<Book>>,
      selected_book: Signal<Option<BookId>>,
      search_query: Signal<String>
- [ ] catalog/ — CardsView, Card, Filters, Sort components
- [ ] reader/ — ReaderView, PdfViewer (renders pages as <img> from base64),
      EpubViewer (renders HTML), Toolbar, HighlightLayer (SVG overlay)
- [ ] annotations/ — Modal, Panel, ConceptList, MarkerItem, TagInput, Autocomplete
- [ ] knowledge_graph/ — GraphView (SVG), GraphNode, GraphPelito, GraphPopup,
      ForceSimulation (port D3 force algo to Rust/WASM)
- [ ] notes/ — NotesView with accordion, search
```

## Phase 10: Testing

```
- [ ] tests/unit/
      catalog_test.rs — filename parser edge cases, scanner logic
      annotations_test.rs — CRUD, autocomplete, concept dedup
      graph_test.rs — bipartite graph building, force layout convergence
      search_test.rs — FTS5 query syntax, ranking
      events_test.rs — EventBus publish/subscribe
- [ ] tests/integration/
      db_test.rs — migrations run cleanly, schema is correct
      catalog_service_test.rs — scan real folder → books in DB
      annotation_service_test.rs — full flow: concept → marker → highlight → search
      migration_test.rs — annotations.json → SQLite migration
- [ ] tests/e2e/
      app_test.rs — Tauri WebDriver:
        open app → see books → click book → reader opens → add concept →
        switch to graph → see node → click pelito → popup shows
```

## Phase 11: Boundary Protection

```
- [ ] tools/arch_rules.toml:
      [rules]
      # Domain modules cannot depend on each other directly
      catalog.cannot_import = ["annotations", "reader", "knowledge_graph", "sync"]
      annotations.cannot_import = ["catalog", "reader", "knowledge_graph", "sync"]
      reader.cannot_import = ["catalog", "annotations", "knowledge_graph", "sync"]
      knowledge_graph.cannot_import = ["reader", "sync"]
      
      # Only core/ and storage/ are shared
      # Modules communicate via events (core/events.rs)
      
      # Frontend mirrors backend structure
      # No frontend module imports another module's internals
      
      [allowed_shared]
      all_modules = ["core", "storage", "observability"]

- [ ] tools/boundary_checker.rs:
      Parses `use` statements in all .rs files
      Validates against arch_rules.toml
      Reports violations with file:line
      Exit code 1 if any violation found

- [ ] .cargo/config.toml — add as pre-build check:
      [alias]
      check-arch = "run --bin boundary_checker"

- [ ] Pre-commit hook:
      cargo check-arch && cargo test && cargo clippy
```

## Phase 12: Build & Package

```
- [ ] cargo tauri build
      → target/release/bundle/dmg/Cognix.dmg      (macOS)
      → target/release/bundle/msi/Cognix.msi      (Windows)
      → target/release/bundle/appimage/Cognix.AppImage (Linux)
- [ ] tauri.conf.json:
      app name, window size, icon, permissions
      auto-update server URL (for future)
- [ ] CI/CD (GitHub Actions):
      on push → cargo check-arch → cargo test → cargo clippy
      on tag → cargo tauri build → upload artifacts → create release
```

---

## Dependencies

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri = { version = "2", features = ["shell-open"] }
rusqlite = { version = "0.32", features = ["bundled", "fts5"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
walkdir = "2"
sha2 = "0.10"
pdfium-render = "0.8"
epub = "2"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
tokio = { version = "1", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }

# src/Cargo.toml (Leptos frontend)
[dependencies]
leptos = "0.7"
leptos_router = "0.7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
wasm-bindgen = "0.2"
web-sys = "0.3"
```

## Execution Order

| Phase | What | Depends on |
|-------|------|-----------|
| 1 | Core infrastructure | — |
| 2 | Storage (SQLite) | Phase 1 |
| 3 | Observability | Phase 1 |
| 4 | Catalog module | Phase 1, 2, 3 |
| 5 | Annotations module | Phase 1, 2, 3 |
| 6 | Reader module | Phase 1, 3 |
| 7 | Knowledge graph | Phase 4, 5 |
| 8 | Search module | Phase 2, 5 |
| 9 | Leptos frontend | Phase 4, 5, 6, 7, 8 |
| 10 | Testing | All phases |
| 11 | Boundary protection | All phases |
| 12 | Build & package | All phases |
