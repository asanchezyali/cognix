# Plan: Sync + Collaborative Spaces for Cognix

## Context

Cognix is currently a local-only app: one user, one machine. The goal is to enable:
1. **Sync**: the same user accesses their library, annotations, and knowledge graph from multiple devices
2. **Collaborative spaces**: multiple users share a library, see each other's annotations, build a shared knowledge graph

This is the step that takes Cognix from a personal tool to a platform.

## Architecture Decision: Supabase

**Why Supabase** over a custom backend:
- Auth (email, GitHub OAuth, magic link) out of the box
- PostgreSQL with Row-Level Security for permissions
- Realtime subscriptions for live collaboration
- Storage (S3-compatible) for PDFs/EPUBs
- Free tier: 500MB database, 1GB storage, 50K monthly active users
- Client SDK works in browser (our frontend is already HTML/JS)
- Self-hostable if we outgrow the free tier or want full control

**What stays local**: the Rust server stays as the local reader/viewer. Supabase handles data sync and collaboration. The app works offline and syncs when online (offline-first).

---

## Step-by-Step Implementation

### Phase 1: Database Schema & Auth (Week 1)

**Step 1.1: Create Supabase project**
- Create project at supabase.com
- Get API URL + anon key
- Store in `.app/config.json` (gitignored)

**Step 1.2: Database schema**

```sql
-- Users (handled by Supabase Auth)

-- Workspaces (personal or shared)
CREATE TABLE workspaces (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name TEXT NOT NULL,
  owner_id UUID REFERENCES auth.users(id),
  is_personal BOOLEAN DEFAULT true,
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Workspace members
CREATE TABLE workspace_members (
  workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE,
  user_id UUID REFERENCES auth.users(id),
  role TEXT CHECK (role IN ('owner', 'editor', 'viewer')) DEFAULT 'viewer',
  joined_at TIMESTAMPTZ DEFAULT now(),
  PRIMARY KEY (workspace_id, user_id)
);

-- Books (catalog)
CREATE TABLE books (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE,
  file_name TEXT NOT NULL,
  author TEXT,
  year INT,
  title TEXT,
  category TEXT,
  path TEXT,
  ext TEXT,
  file_hash TEXT,            -- SHA-256 for dedup
  file_size BIGINT,
  storage_path TEXT,         -- path in Supabase Storage
  uploaded_by UUID REFERENCES auth.users(id),
  created_at TIMESTAMPTZ DEFAULT now(),
  UNIQUE(workspace_id, file_name)
);

-- Concepts
CREATE TABLE concepts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  workspace_id UUID REFERENCES workspaces(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  created_by UUID REFERENCES auth.users(id),
  created_at TIMESTAMPTZ DEFAULT now(),
  UNIQUE(workspace_id, name)
);

-- Book-Concept links
CREATE TABLE book_concepts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  book_id UUID REFERENCES books(id) ON DELETE CASCADE,
  concept_id UUID REFERENCES concepts(id) ON DELETE CASCADE,
  created_by UUID REFERENCES auth.users(id),
  UNIQUE(book_id, concept_id)
);

-- Markers (notes linked to a concept in a book)
CREATE TABLE markers (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  book_concept_id UUID REFERENCES book_concepts(id) ON DELETE CASCADE,
  page TEXT,
  note TEXT,
  created_by UUID REFERENCES auth.users(id),
  created_at TIMESTAMPTZ DEFAULT now(),
  updated_at TIMESTAMPTZ DEFAULT now()
);

-- Highlights (visual selections in PDFs)
CREATE TABLE highlights (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  marker_id UUID REFERENCES markers(id) ON DELETE CASCADE,
  page INT,
  text TEXT,
  rects JSONB,              -- [{x,y,w,h}, ...]
  created_by UUID REFERENCES auth.users(id),
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Tags
CREATE TABLE tags (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  book_id UUID REFERENCES books(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  created_by UUID REFERENCES auth.users(id),
  UNIQUE(book_id, name)
);
```

**Step 1.3: Row-Level Security (RLS)**

```sql
-- Users can only see workspaces they belong to
ALTER TABLE workspaces ENABLE ROW LEVEL SECURITY;
CREATE POLICY "workspace_access" ON workspaces
  FOR ALL USING (
    id IN (SELECT workspace_id FROM workspace_members WHERE user_id = auth.uid())
  );

-- Books visible to workspace members
ALTER TABLE books ENABLE ROW LEVEL SECURITY;
CREATE POLICY "book_access" ON books
  FOR SELECT USING (
    workspace_id IN (SELECT workspace_id FROM workspace_members WHERE user_id = auth.uid())
  );
CREATE POLICY "book_write" ON books
  FOR ALL USING (
    workspace_id IN (SELECT workspace_id FROM workspace_members WHERE user_id = auth.uid() AND role IN ('owner', 'editor'))
  );

-- Same pattern for concepts, markers, highlights, tags
-- Viewers can read, editors can write, owners can delete
```

**Step 1.4: Auth in the frontend**

```js
// .app/library.html - add Supabase client
import { createClient } from 'https://cdn.jsdelivr.net/npm/@supabase/supabase-js/+esm'

const supabase = createClient(SUPABASE_URL, SUPABASE_ANON_KEY)

// Login screen before library loads
async function checkAuth() {
  const { data: { user } } = await supabase.auth.getUser()
  if (!user) showLoginScreen()
  else loadWorkspaces()
}
```

Login options: email + password, GitHub OAuth, magic link.

---

### Phase 2: Sync Engine (Week 2)

**Step 2.1: Sync annotations up**

When user creates/edits a concept, marker, or highlight locally:
1. Write to local `.data/annotations.json` (immediate, offline-first)
2. Queue the change for sync
3. Push to Supabase when online

```js
class SyncEngine {
  queue = []
  
  async push(operation) {
    // Save locally first
    this.saveLocal()
    // Queue for remote
    this.queue.push(operation)
    if (navigator.onLine) await this.flush()
  }
  
  async flush() {
    while (this.queue.length) {
      const op = this.queue[0]
      try {
        await supabase.from(op.table).upsert(op.data)
        this.queue.shift()
      } catch (e) {
        break // retry later
      }
    }
  }
}
```

**Step 2.2: Sync annotations down**

On app load:
1. Load local data (instant)
2. Fetch remote changes since last sync timestamp
3. Merge (remote wins for conflicts, or show conflict UI)

```js
async function pullChanges() {
  const lastSync = localStorage.getItem('lastSync') || '1970-01-01'
  const { data } = await supabase
    .from('markers')
    .select('*')
    .gt('updated_at', lastSync)
  // Merge into local state
  mergeRemoteData(data)
  localStorage.setItem('lastSync', new Date().toISOString())
}
```

**Step 2.3: File sync (PDFs/EPUBs)**

PDFs are large — don't sync all files to all devices. Instead:
- Upload: when user adds a book, upload to Supabase Storage
- Download on demand: when user opens a book on another device, stream from Storage
- Cache: keep downloaded PDFs in a local cache dir
- Hash dedup: same file (by SHA-256) is stored once

```
Supabase Storage
  /workspaces/{workspace_id}/books/{file_hash}.{ext}
```

The Rust server gets a new endpoint:
```
GET /api/file/{book_id} → serves from local cache or proxies from Supabase Storage
```

**Step 2.4: Offline queue persistence**

Store sync queue in IndexedDB so it survives browser refresh:
```js
// Use idb-keyval (tiny IndexedDB wrapper)
import { get, set } from 'https://cdn.jsdelivr.net/npm/idb-keyval/+esm'
```

---

### Phase 3: Collaborative Workspaces (Week 3)

**Step 3.1: Workspace UI**

Add a workspace switcher to the header:
```
[Personal ▼] Library
```

Dropdown shows:
- Personal workspace (created on signup)
- Shared workspaces the user belongs to
- "+ Create workspace" button

**Step 3.2: Create/join workspace**

```js
// Create
const { data: ws } = await supabase.from('workspaces')
  .insert({ name: 'ML Reading Group', owner_id: user.id, is_personal: false })
  .select().single()

// Invite by email
await supabase.from('workspace_members')
  .insert({ workspace_id: ws.id, user_id: inviteeId, role: 'editor' })
```

Invite flow: owner enters email → if user exists, add directly. If not, send invite link (Supabase Auth handles this).

**Step 3.3: Shared annotations**

In a shared workspace, all members see:
- All books (uploaded by any member)
- All concepts, markers, highlights (created by any member)
- Who annotated what (avatar/initials next to each annotation)

```sql
-- Add user info to marker display
SELECT m.*, u.email, u.raw_user_meta_data->>'name' as author_name
FROM markers m
JOIN auth.users u ON m.created_by = u.id
WHERE m.book_concept_id IN (...)
```

**Step 3.4: Realtime collaboration**

When two users are reading the same book simultaneously:
- See each other's cursor/page position
- New highlights appear in real-time
- New markers appear in the panel live

```js
// Subscribe to changes in the current workspace
supabase
  .channel('workspace:' + workspaceId)
  .on('postgres_changes', { 
    event: '*', 
    schema: 'public', 
    table: 'markers',
    filter: `book_concept_id=in.(${bookConceptIds})`
  }, (payload) => {
    // Update local state and re-render
    handleRealtimeChange(payload)
  })
  .subscribe()
```

**Step 3.5: Presence**

Show who's online and what they're reading:
```js
supabase.channel('workspace:' + workspaceId)
  .on('presence', { event: 'sync' }, () => {
    const state = channel.presenceState()
    renderOnlineUsers(state)
  })
  .track({ user_id: user.id, reading: currentBook?.title })
```

UI: small avatars in the header showing who's online. If someone is reading the same book, show their avatar in the reader bar.

---

### Phase 4: Permissions & Sharing (Week 4)

**Step 4.1: Role-based access**

| Action | Owner | Editor | Viewer |
|--------|-------|--------|--------|
| Upload books | yes | yes | no |
| Delete books | yes | no | no |
| Add concepts/markers | yes | yes | no |
| Edit own annotations | yes | yes | yes (own only) |
| Delete annotations | yes | own only | own only |
| Invite members | yes | no | no |
| Change roles | yes | no | no |
| Delete workspace | yes | no | no |

**Step 4.2: Share a concept/note publicly**

"Share this concept" → generates a public URL:
```
https://cognix.app/shared/concept/{uuid}
```
Shows the concept with all its markers across books (read-only). No auth needed. Good for sharing on Twitter, blogs, etc.

**Step 4.3: Export**

- Export workspace annotations as Markdown
- Export knowledge graph as JSON/image
- Export highlights as PDF annotations (write back into the PDF)

---

### Phase 5: Tauri Migration (Week 5-6)

**Step 5.1: Why Tauri**
- Native app (.dmg, .exe, .AppImage)
- Access to filesystem without browser restrictions
- System tray, global shortcuts
- Auto-updates
- Already have Rust backend + HTML/JS frontend = perfect match

**Step 5.2: Project structure**

```
cognix/
  src-tauri/
    src/main.rs          # Tauri app + merged with current server logic
    Cargo.toml
    tauri.conf.json
  src/
    index.html           # Current library.html
    pdfjs/
  package.json
```

**Step 5.3: Migration steps**
1. `npm create tauri-app@latest` scaffold
2. Move `.app/library.html` → `src/index.html`
3. Move server logic into Tauri commands (filesystem scan, data persistence)
4. Supabase client stays in JS frontend
5. PDF.js viewer embedded as web asset
6. Build: `cargo tauri build` → .dmg

---

## File Changes Summary

| Phase | Files |
|-------|-------|
| 1 | New: `.app/supabase.js` (client), DB migrations |
| 2 | Modified: `.app/library.html` (sync engine), `.app/server/src/main.rs` (file proxy) |
| 3 | Modified: `.app/library.html` (workspace UI, realtime) |
| 4 | Modified: `.app/library.html` (permissions UI, share links) |
| 5 | New: `src-tauri/`, restructured from `.app/` |

## Tech Stack Final

| Layer | Technology |
|-------|-----------|
| Frontend | HTML/JS (single file), D3.js, epub.js, PDF.js |
| Local server | Rust (axum) → Tauri commands |
| Database | Supabase PostgreSQL |
| Auth | Supabase Auth (email, GitHub, magic link) |
| Storage | Supabase Storage (PDFs) |
| Realtime | Supabase Realtime (Postgres changes + Presence) |
| Sync | Offline-first with IndexedDB queue |
| Desktop | Tauri (Rust + WebView) |

## Verification per Phase

1. **Auth**: signup → login → see personal workspace
2. **Sync**: create annotation on device A → appears on device B after refresh
3. **Collab**: user A highlights text → user B sees it in real-time
4. **Permissions**: viewer cannot add annotations, editor can, owner can delete
5. **Tauri**: `cargo tauri build` → .dmg installs and runs on fresh Mac
