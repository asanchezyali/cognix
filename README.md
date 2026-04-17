# Cognito

A local-first personal library browser with a concept-based knowledge graph. Read, annotate, and connect ideas across your PDFs and EPUBs.

## What it does

- **Organize** a collection of PDFs and EPUBs into a searchable, filterable catalog
- **Read** documents in-app with a split view: book on the left, notes on the right
- **Annotate** with concepts, markers (page + note), and text highlights
- **Connect** ideas across books through a visual knowledge graph
- **Navigate** your annotations: click a marker to jump to that page in the book

## The knowledge graph

Cognito builds a bipartite graph of your reading:

- **Squares** = books (colored by category)
- **Circles** = concepts you defined (e.g., "reinforcement-learning", "metric-space")
- **Dots** = individual markers/notes orbiting each concept
- **Lines** = a book contains that concept

Hover to highlight connections. Click a marker dot to see the full note and jump to the page.

## Features

- **Cards view** with search, filters (category, year, format, tags)
- **Graph view** with the bipartite knowledge graph
- **Notes view** with all your annotations across all books, searchable
- **PDF reader** with text selection, highlights, and page navigation
- **EPUB reader** with dark theme, code block detection, continuous scroll
- **Split panel** for taking notes while reading
- **Concept autocomplete** suggests existing concepts as you type
- **Offline-first** all data stored locally in JSON

## Tech stack

| Layer | Technology |
|-------|-----------|
| Server | Rust (axum) |
| Frontend | Single HTML file, vanilla JS |
| PDF viewer | PDF.js with custom text layer |
| EPUB viewer | epub.js |
| Knowledge graph | D3.js force-directed |
| Data | JSON files on disk |

## Getting started

### Prerequisites

- Rust toolchain (`rustup`)
- A folder with PDFs/EPUBs

### Install

```bash
git clone https://github.com/asanchezyali/cognito.git
cd cognito

# Build the server
cd .app/server && cargo build --release && cd ../..
```

### Run

```bash
# Point it at your library folder
PORT=8384 .app/server/target/release/library-server /path/to/your/books

# Or use the launcher (opens browser automatically)
.app/open-library.sh
```

Open `http://localhost:8384` in your browser.

### Adding books

Drop PDFs or EPUBs into any subfolder of your library directory. The server scans the filesystem on each request to `/api/catalog` -- no manual indexing needed.

For best results, use the naming convention:

```
Author.Year.Title.Category.ext
```

Example: `SuttonAndBarto.2018.ReinforcementLearningAnIntroduction.MachineLearning.pdf`

But any filename works. The server extracts what it can and falls back to the raw filename.

## Project structure

```
your-library/
  .app/                     # Application code
    library.html            # Single-file frontend
    open-library.sh         # Launch script
    server/                 # Rust server (axum)
    pdfjs/                  # PDF.js viewer with text layer
  .data/                    # User data (gitignored)
    annotations.json        # Concepts, markers, highlights, tags
  .gitignore
  ComputerScience/          # Your book folders
  Mathematics/
  ...
```

## API

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/catalog` | GET | Returns all books found in the library |
| `/api/data` | GET | Returns annotations (concepts, markers, highlights) |
| `/api/data` | POST | Saves annotations |
| `/open/{path}` | GET | Opens a file with the native OS app |

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full plan including:

- Sync between devices (Supabase)
- Collaborative workspaces
- AI-powered concept suggestions
- Full-text search across PDFs
- Tauri desktop app (.dmg, .exe)

## License

[MIT](LICENSE)
