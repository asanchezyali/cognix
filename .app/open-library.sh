#!/bin/bash
# Launch the library browser
LIBRARY_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$LIBRARY_ROOT/.app/server/target/release/library-server"
PORT=8384

# Build if binary doesn't exist
if [ ! -f "$BIN" ]; then
  echo "Building server..."
  (cd "$LIBRARY_ROOT/.app/server" && cargo build --release)
fi

# Kill any previous instance
lsof -ti:$PORT | xargs kill -9 2>/dev/null

echo "Library at http://localhost:$PORT"
open "http://localhost:$PORT"
PORT=$PORT "$BIN" "$LIBRARY_ROOT"
