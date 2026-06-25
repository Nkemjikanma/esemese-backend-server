# Esemese Execution Plan

Photography portfolio with a Dropbox-inspired backend: Rust server generates presigned URLs, client uploads directly to RustFS (S3-compatible, self-hosted on the same VPS), and PostgreSQL tracks metadata. TanStack Start frontend with SSR for the public gallery, protected admin area for uploads. All services run on a single Hetzner VPS. If storage needs outgrow the VPS disk, swap the RustFS endpoint for Hetzner Object Storage — zero code changes.

---

## Phase 0: Infrastructure & Tooling

### 0.1 — Hetzner VPS Setup
- [x] Provision VPS (CX22 or CX32 — CX32 recommended since you're running everything on one box: RustFS + Postgres + backend + frontend)
- [x] Set up firewall (only 80/443/22), fail2ban, non-root user, SSH keys
- [x] Install Docker (for PostgreSQL, RustFS, and app services)

### 0.2 — PostgreSQL
- [ ] Run Postgres (Docker or managed) - docker
- [ ] Install `sqlx-cli` and set up migration workflow (`sqlx migrate add`, `sqlx migrate run`)
- [ ] Set up SQLx compile-time query checking — run `cargo sqlx prepare` for offline mode so CI builds work without a live DB

### 0.3 — RustFS (S3-compatible, self-hosted on VPS)
- [ ] Run RustFS in Docker on the VPS (image: `rustfs/rustfs:latest`, same Docker Compose as Postgres)
  - RustFS runs as non-root (UID 10001) — you must `chown -R 10001:10001` on mounted data/logs volumes
- [ ] Configure RustFS data directory to a dedicated path on the VPS disk (e.g., `/data/rustfs`) — keep it separate from the OS and app files so you can monitor disk usage easily
- [ ] Create a bucket for photos (e.g., `esemese-photos`)
- [ ] Create a second bucket for thumbnails (e.g., `esemese-thumbnails`) — or use a prefix in the same bucket
- [ ] Set up RustFS access credentials (access key + secret key) via environment variables or the web console
- [ ] Note your RustFS S3 endpoint: `http://rustfs:9000` within Docker network, or `http://localhost:9000` from the host
- [ ] Configure bucket policies — photo bucket should be **private** (presigned URLs grant temporary access), but consider making thumbnails **public** for faster gallery loads without presigned URLs
- [ ] **Do NOT expose RustFS port (9000/9001) to the internet** — it only needs to be reachable within the Docker network. The browser uploads to RustFS via presigned URLs, which go through your Caddy reverse proxy.
- [ ] Set up RustFS console (port 9001) for your own admin use — optionally expose behind Caddy with auth, or just SSH tunnel when you need it

**Think about:**
- RustFS is written in Rust and claims ~2.3x faster than MinIO for small objects. It's a drop-in replacement — same ports (9000/9001), same S3 API, same `aws-sdk-s3` client config.
- RustFS is still in alpha (v1.0.0-alpha.90 as of March 2026). For your single-user portfolio this is fine. If you hit an S3 compatibility bug, you can swap to MinIO by changing the Docker image — same ports, same API, same data format.
- RustFS uses less RAM than MinIO. On a CX32 (8GB RAM), that's fine alongside Postgres, your Rust server, and the Node frontend.
- Your current photo needs (~5-10GB) fit easily on the VPS disk. Monitor with a simple disk usage alert. If you outgrow the disk, the escape hatch is swapping the S3 endpoint in your config to Hetzner Object Storage — zero code changes.
- **Presigned URLs and the browser:** The presigned URL your server generates contains the RustFS endpoint. For the browser to PUT directly to RustFS, that endpoint must be reachable from the browser. This means you need to proxy RustFS's S3 API through Caddy (e.g., `s3.esemese.com` → RustFS port 9000). Only the S3 API port, not the console.

### 0.4 — Domain & TLS
- [ ] Point domain at the VPS
- [ ] Set up reverse proxy (Caddy recommended — automatic HTTPS)
- [ ] Plan subdomains: `esemese.com` (frontend), `api.esemese.com` (backend), `s3.esemese.com` (RustFS S3 API — needed so the browser can PUT to presigned URLs)

---

## Phase 1: Backend Core (esemese-backend-server)

### 1.1 — Application Bootstrap
- [x] Wire up `actix-web` startup in `startup.rs` — server config, middleware stack, route registration
- [x] Load config from environment into an `AppConfig` struct (DB URL, RustFS S3 endpoint + credentials, public S3 URL for presigned URLs, JWT secret, allowed origins)
  - You'll need two S3 URLs: the **internal** endpoint for server→RustFS calls (`http://rustfs:9000` within Docker) and the **public** endpoint for presigned URLs the browser uses (`https://s3.esemese.com`)
- [x] Set up `tracing-subscriber` for structured logging
- [x] Set up CORS properly from the start (configurable origins)

### 1.2 — Database Schema
- [x] Design and write migrations for core tables:
  - **`photos`** — id (UUID), title, description, category, featured, original_filename, s3_key (the **private original**), file_size, mime_type, width, height, **blurhash** (LQIP placeholder), **status** (`processing` → `ready` → `failed`), created_at, updated_at
  - **`photo_variants`** (NEW — replaces the fixed `thumbnail_s3_key_*` columns) — id, photo_id (FK, CASCADE), s3_key (the **public derivative**), width, height, format (`avif`/`webp`/`jpeg`), byte_size. One row per `(photo_id, width, format)`. Unique on `(photo_id, width, format)`. This is what the gallery serves; adding a new width later is data, not a migration.
  - **`photo_metadata`** — photo_id (FK), camera, lens, iso, aperture, shutter_speed, focal_length, location, taken_at
  - **`collections`** — id, name, slug, description, cover_photo_id (FK), created_at
  - **`collection_photos`** — collection_id, photo_id, sort_order (join table, a photo can be in multiple collections)
- [x] Run migrations and verify schema

**Why this shape (decided 2026-06-08):** A portfolio is read-heavy and the gallery images are public. The original is the only private asset; everything a visitor sees is a regenerable, cacheable derivative. Modelling derivatives as a table (not 2 fixed columns) lets you serve a responsive `srcset` ladder in modern formats — see 1.5/1.7.

**Think about:**
- **`photos.status`** exists because derivatives are generated **async** (1.7) — a photo row is created at `confirm` *before* its variants encode. The public gallery must filter to `status = 'ready'` so it never renders a photo with no variants yet.
- **`blurhash`** is computed once during processing and stored as a short string — the frontend renders it instantly while the real image loads.
- Do you want categories as a column on `photos` or as a separate `categories` table with a join? Separate table is more flexible but a simple text column works fine for a single-user portfolio.
- EXIF data: extract this **server-side** from the uploaded original (`kamadak-exif` crate) during processing — never trust client-supplied metadata. Strip GPS from public derivatives (see 1.7) but you may keep `taken_at`/camera/lens in `photo_metadata`.

### 1.3 — Authentication
- [x] Seed admin user via migration or first-run setup — **single admin, env-based** (`ADMIN_USERNAME` + `ADMIN_PASSWORD_HASH`), no `users` table
- [x] Implement `POST /auth/login` — takes username/password, returns JWT
- [x] Hash passwords with Argon2
- [x] Build JWT middleware on actix-web that protects admin routes
- [x] Decide on token strategy — **single access token, 1h expiry, no refresh tokens.** Rotate `JWT_SECRET` for "logout everywhere".

### 1.4 — Presigned URL Upload Flow (the Dropbox part)

**Single-file lifecycle:**
1. Client requests upload → `POST /uploads/initiate` (authed, sends filename, content type, file size)
2. Server generates presigned PUT URL → returns it to the client along with an `upload_id`
3. Client uploads directly to S3 using the presigned URL (PUT request with file body)
4. Client confirms upload → `POST /uploads/confirm` (sends `upload_id`, metadata)
5. Server verifies the object exists in S3, creates the DB record, triggers thumbnail generation

**Multi-file upload strategy:**

The server stays simple — `POST /uploads/initiate` handles one file at a time. No batch endpoint needed. The client orchestrates multi-file uploads using a **queue with concurrent workers:**

- Client maintains an upload queue of selected files
- N workers (e.g., 3) pull files from the queue concurrently
- Each worker runs the full single-file lifecycle independently
- **Just-in-time initiation** — only request the presigned URL when a file is about to start uploading, NOT upfront for all files. This avoids presigned URLs expiring while files sit in the queue waiting their turn.
- Per-file confirm — each file is confirmed independently as it finishes. A slow or failed upload doesn't block the others.

**Per-file state machine** (tracked client-side):
```
pending → initiating → uploading → confirming → done
                ↓            ↓           ↓
              failed       failed      failed
```

**Per-file retry** — if a file fails at any stage, retry just that file (re-initiate to get a fresh presigned URL). Don't restart the whole batch. If S3 returns 403 during upload, the presigned URL likely expired — re-initiate and retry the upload.

**Backend tasks:**
- [x] Implement `POST /uploads/initiate` — validate file type/size, generate S3 key, create presigned PUT URL, store pending upload in DB
- [x] Implement `POST /uploads/confirm` — verify object exists in S3, save photo record + metadata to DB
- [x] Add upload state tracking in DB (initiated, uploaded, confirmed, failed)
- [ ] Add cleanup job for stale initiated uploads (e.g., cron or background task that deletes uploads stuck in "initiated" for more than 1 hour — also delete the orphaned S3 object if it exists)

**Upload method: presigned PUT, size enforced at confirm (decided 2026-06-08).** We chose PUT over POST: the Rust `aws-sdk-s3` gives PUT presigning in one call but has **no presigned-POST helper** (you'd hand-roll the SigV4 POST policy). POST's only real win — `content-length-range` enforced at the storage edge — is low value for a single-admin tool where the client is you. Revisit POST only if uploads ever open to other users.

**Don't trust the client at the upload boundary:** A presigned PUT URL is a capability you hand to a browser. With PUT you can pin the **exact content-type** on the signed URL, but **not a size range** — so size is enforced server-side at `confirm`.
- **Pin content-type on the presigned PUT**; the client must send the matching `Content-Type` header or the signature fails.
- **Enforce size at `confirm`** — read the object's `ContentLength` back and reject if outside `[MIN, MAX]`. Nearly free, since confirm already reads the object to validate it.
- **Re-derive, never trust, on confirm** — read the actual object back: verify magic bytes are really JPEG/PNG/WebP (not just the extension), decode true width/height yourself, extract EXIF yourself. Persist only server-derived values.
- **Decode-bomb defense** — cap decoded dimensions before processing (the `image` crate supports limits); a 100MP "pixel flood" PNG can OOM the box.
- **Key originals by `photo_id` (UUID), NOT filename or content hash** (decided 2026-06-08). `originals/{photo_id}` is unique by construction, kills the `file_name`-collision bug with zero client work, and satisfies `photos.s3_key UNIQUE`. We rejected content-hashing the original: its only wins are dedupe (a single photographer never needs it) and immutability (irrelevant for a private, rarely-fetched original), and it would force the browser to hash every file before initiate — added latency + Web Worker complexity for no payoff. **Immutability is recovered where it actually matters — the public variants — by content-hashing them server-side (the worker already has the bytes, so it's free; see 1.7).**

**Things to think about:**
- **Presigned URL expiry** — 5-15 minutes is typical. With just-in-time initiation you only need enough time for one file's upload, so shorter TTLs are safe.
- **For large files (chunked/multipart upload):** `CreateMultipartUpload` → presigned URLs per part → `CompleteMultipartUpload`. For photos (5-30MB), single presigned PUT is fine. Optional learning exercise for files over ~10MB.
- **The S3 Rust SDK:** `aws-sdk-s3` works with RustFS (S3-compatible). Fallback: swap the image to `minio/minio` if you hit edge cases — zero code changes.

### 1.5 — Serving Flow (two tiers — decided 2026-06-08)

A portfolio is the inverse of Dropbox: the gallery is *meant* to be seen by everyone. Presigning public content fights the use case (breaks CDN caching, adds a round-trip per thumbnail). So split serving by visibility:

| Asset | Visibility | How it's served |
|---|---|---|
| **Variants** (gallery thumbnails + web-sized display images) | **Public** | Stable public URL → **CDN/Caddy** → `Cache-Control: public, max-age=31536000, immutable`. No presign. The frontend builds `srcset` directly from `photo_variants` keys. |
| **Original master** | **Private** | `GET /photos/:id/original-url` → presigned GET, short TTL. *This* is where the Dropbox pattern legitimately belongs — only used when someone explicitly downloads the full-res file. |

- [ ] Public variant URLs: serve the derivative bucket/prefix through Caddy with long immutable cache headers (content-addressed keys make `immutable` safe).
- [ ] `GET /photos/:id/original-url` — presigned GET for the original, admin-only or gated, short TTL.
- [ ] List/detail endpoints return variant keys (or full public URLs) so the frontend can assemble `srcset` without N presign calls.

**Why no hour-boundary presign hack anymore:** that trick existed only to make presigned URLs cacheable. With public immutable variant URLs the problem disappears — the CDN caches them forever and you never sign a request for content that isn't secret.

### 1.6 — CRUD Endpointvs

**Public (no auth):**
- [ ] `GET /photos` — list photos (paginated, filterable by category/collection)
- [ ] `GET /photos/:id` — single photo with metadata
- [ ] `GET /collections` — list collections
- [ ] `GET /collections/:slug` — collection detail with photos
- [ ] `GET /photos/:id/original-url` — presigned URL for the **private original** only (from 1.5). Public variant URLs come back inline on the list/detail responses — no per-image presign.

**Admin (authed):**
- [x] `POST /uploads/initiate` — start upload (from 1.4)
- [x] `POST /uploads/confirm` — confirm upload + save metadata (from 1.4)
- [ ] `PUT /photos/:id` — update photo metadata
- [ ] `DELETE /photos/:id` — delete photo (also delete from S3)
- [ ] `POST /collections` — create collection
- [ ] `PUT /collections/:id` — update collection
- [ ] `DELETE /collections/:id` — delete collection
- [ ] `POST /collections/:id/photos` — add photos to collection
- [ ] `DELETE /collections/:id/photos/:photo_id` — remove photo from collection

### 1.7 — Derivative Generation (the real engineering of this system)

Dropbox's hard problem is sync/chunking; *this* system's hard problem is serving the right pixels fast. Invest here.

- [x] Generate a **responsive ladder**, not 2 sizes — e.g. widths 400 / 800 / 1200 / 1600 / 2400. Serving a 24MP original into a 300px grid cell is the #1 portfolio perf sin.
- [x] Generate **modern formats** — AVIF (best compression) with WebP fallback; can cut bytes 50-70% vs JPEG.
- [x] Compute a **blurhash/LQIP** once and store on `photos.blurhash` — makes the gallery feel instant.
- [ ] **Bake in orientation** and **strip GPS/EXIF** from public derivatives (don't publish your home coordinates).
- [x] Write one `photo_variants` row per `(photo_id, width, format)`; upload each to the public derivative prefix (content-addressed key).
- [x] Flip `photos.status` → `ready` only once the variant set is complete.

**Do it async (decided 2026-06-08):** Encoding a 5-width × 2-format ladder from a 40MB RAW is seconds of CPU — don't block the `confirm` response on it. Pattern: `confirm` creates the `photos` row in `processing`, enqueues a job, returns immediately; a background worker encodes, writes variants, sets `ready`. For a single-user portfolio an in-process `tokio` worker + a `jobs` table is a perfectly legitimate "queue" — you do **not** need Kafka (that's the Dropbox-scale event bus on your whiteboard, not this).

**Source of truth:** object storage owns the *bytes*, Postgres owns the *metadata*. A reconciliation/cleanup job closes the gap both ways — orphaned objects with no row get deleted; rows stuck in `processing` past a timeout get retried or flagged. (Polling-on-confirm is fine; S3 event notifications are the gold-standard upgrade.)

---

## Phase 2: Frontend (TanStack Start)

### 2.1 — Scaffold TanStack Start Project
- [ ] Create a new TanStack Start project (separate from old frontend, or in-place migration)
- [ ] Set up Tailwind CSS
- [ ] Set up TanStack Query

**Note:** TanStack Start is still relatively new. Read the docs carefully — it uses Vinxi under the hood, has file-based routing (which you're already familiar with from TanStack Router), and adds SSR + server functions.

### 2.2 — Route Structure
- [ ] Set up route structure:
  ```
  routes/
  ├── __root.tsx              # Root layout (nav, footer)
  ├── index.tsx               # Landing page / hero
  ├── gallery/
  │   ├── index.tsx           # All collections grid
  │   └── $slug.tsx           # Single collection view
  ├── photo/
  │   └── $id.tsx             # Individual photo detail + lightbox
  ├── about.tsx               # About page
  ├── admin/
  │   ├── login.tsx           # Admin login
  │   ├── route.tsx           # Admin layout (auth guard)
  │   ├── upload.tsx          # Upload interface
  │   ├── photos.tsx          # Manage photos
  │   └── collections.tsx     # Manage collections
  ```

### 2.3 — SSR & Data Loading
- [ ] Implement route loaders for public gallery pages (SSR for SEO)
- [ ] Admin pages: client-side only (no SSR needed)
- [ ] Use TanStack Start server functions for server-side API calls (avoids CORS for SSR requests; client-side hydration calls API directly)

### 2.4 — Upload UX
- [ ] Multi-file select with drag & drop zone
- [ ] Per-file: show preview (`URL.createObjectURL`), metadata form (title, description, category)
- [ ] Implement client-side upload queue with concurrent workers (3 max)
- [ ] Per-file state machine in UI: pending → initiating → uploading → confirming → done/failed
  - Show status indicator per file (spinner, progress bar, checkmark, error icon)
- [ ] Just-in-time presigned URL: each worker calls `POST /uploads/initiate` only when it dequeues a file to upload — not upfront for all files
- [ ] Upload flow per worker:
  1. Dequeue next file from queue
  2. Call `POST /uploads/initiate` (get fresh presigned URL)
  3. PUT file directly to S3 from the browser (with progress tracking)
  4. Call `POST /uploads/confirm` with metadata
  5. Pick up next file from queue
- [ ] Per-file retry on failure — re-initiate to get a fresh presigned URL, retry the upload. Don't restart the whole batch.
- [ ] Handle S3 403 during upload (expired presigned URL) — re-initiate and retry automatically

**Think about:** `XMLHttpRequest` gives you `upload.onprogress` for real progress bars. `fetch` doesn't have native upload progress. For the progress bar, XHR is the pragmatic choice.

### 2.5 — Gallery UX
- [ ] Collections grid with thumbnails (public/presigned thumbnail URLs)
- [ ] Masonry or grid layout for collection detail
- [ ] Lightbox for full-res viewing (generate presigned URL on demand)
- [ ] EXIF data display on photo detail page
- [ ] Virtual scrolling for large collections (`@tanstack/react-virtual`)
- [ ] Image loading: `loading="lazy"`, consider blur-up placeholders (store a tiny base64 blur hash in the DB)

### 2.6 — Drop Wagmi/Web3
- [ ] Remove all Wagmi, Viem, Porto dependencies
- [ ] Replace wallet-based identity with JWT auth for admin
- [ ] Clean up wallet-connected routes

---

## Phase 3: Deployment & Operations

### 3.1 — Containerization
- [x] Dockerfile for Rust backend (multi-stage build: builder stage with cargo, slim runtime stage)
- [x] Dockerfile for TanStack Start frontend (Node runtime)
- [x] Docker Compose for all environments: backend + postgres + rustfs + frontend
  - Same compose structure in dev and prod — difference is just env vars (ports, domains, credentials)

### 3.2 — Deployment on Hetzner
- [ ] Configure reverse proxy (Caddy or Nginx):
  - `esemese.com` → TanStack Start (port 3000)
  - `api.esemese.com` → Rust backend (port 8080)
  - `s3.esemese.com` → RustFS S3 API (port 9000) — required for browser presigned URL uploads
- [ ] Run all services via Docker Compose on the VPS (Postgres, RustFS, backend, frontend)
- [ ] Set up CI/CD (simple option: SSH + pull + rebuild on push, or GitHub Actions)
- [ ] Set up disk usage monitoring/alerting — you're sharing the VPS disk between OS, containers, Postgres, and RustFS data

### 3.3 — Backup Strategy
- [ ] PostgreSQL: `pg_dump` on a cron, store backups in a RustFS backup bucket
- [ ] RustFS data: periodic rsync or rclone to an off-VPS location (your local machine, a cheap storage VPS, or a cloud storage free tier)
- [ ] **Your photos are irreplaceable** — the VPS disk is a single point of failure. Keep at least one off-site copy of your originals. Even a manual `rclone sync` to your local machine on a schedule works.
- [ ] Consider Hetzner Storage Box (~€3/mo for 1TB) as a cheap off-site backup target for both DB dumps and photo originals

---

## Phase 4: Polish & Extras

- [ ] **SEO:** Open Graph tags, structured data (JSON-LD for photographs), sitemap generation
- [ ] **Performance:** Caching headers via Caddy, HTTP/2, compression
- [ ] **EXIF extraction:** Parse EXIF from uploads server-side (`kamadak-exif` crate) so you don't need manual metadata entry
- [ ] **Search:** Full-text search over photo titles/descriptions (Postgres `tsvector`)
- [ ] **Analytics:** Simple hit counter per photo, or self-hosted analytics (Umami, Plausible)

---

## Phase 5: CLI Sync Tool (`esemese-cli`)

A Rust CLI that syncs a local photo directory to your portfolio. Folder = collection. You organize photos on your machine, run the CLI, and the portfolio updates. This also means your local machine is always a copy of your photos.

### 5.1 — Project Setup
- [ ] Create a new Rust binary crate (`esemese-cli`) in the repo — separate from the backend, but can share types via a shared crate if you want
- [ ] Dependencies: `clap` (CLI args), `reqwest` (HTTP client), `aws-sdk-s3` (presigned URL uploads), `serde`/`serde_json`, `tokio`, `walkdir` (directory traversal)
- [ ] Config: reads API URL, auth credentials from a config file (e.g., `~/.config/esemese/config.toml`) or environment variables

### 5.2 — Authentication
- [ ] CLI calls `POST /auth/login` with your admin credentials, gets a JWT
- [ ] Cache the JWT locally (write to config dir) so you don't login every run
- [ ] Handle token expiry — if an API call returns 401, re-login automatically

### 5.3 — Directory Scanning & Diffing
- [ ] Scan the specified local directory — each subfolder is a collection, files inside are photos
  ```
  ~/Photos/portfolio/
  ├── street-lagos/          → collection "street-lagos"
  │   ├── img_001.jpg
  │   ├── img_002.jpg
  │   └── collection.toml    → optional metadata (description, cover photo, sort order)
  ├── portraits-2025/        → collection "portraits-2025"
  │   ├── img_010.jpg
  │   └── img_011.jpg
  ```
- [ ] Fetch current state from the API: `GET /collections` + photos per collection
- [ ] Diff local vs remote:
  - **New folder locally** → create collection + upload all photos
  - **New files in existing folder** → upload new photos to existing collection
  - **File deleted locally** → mark for deletion on server (with confirmation, see below)
  - **Folder deleted locally** → mark collection for deletion (with confirmation)
  - **File exists in both** → skip (already uploaded)
- [ ] **How to detect "already uploaded":** Compare by filename + file size, or store a local state file (e.g., `.esemese-sync.json` in the photos directory) that tracks what's been synced with remote IDs

### 5.4 — Upload Flow
- [ ] Reuse the same presigned URL flow as the browser: initiate → PUT to S3 → confirm
- [ ] Same concurrent upload queue as the browser — 3 workers pulling from a queue
- [ ] Show progress in the terminal: per-file progress bars (`indicatif` crate is great for this)
- [ ] Per-file retry on failure, same as browser flow

### 5.5 — Deletion Handling
- [ ] **Default: dry-run deletes** — show what would be deleted, require `--apply-deletes` flag to actually delete
- [ ] Alternatively: `esemese sync` only uploads/creates, `esemese prune` handles deletes as a separate command
- [ ] Server-side: consider soft-delete (mark hidden, actually purge after 30 days) so accidental `rm -rf` doesn't nuke your portfolio instantly

### 5.6 — Optional Metadata
- [ ] Support an optional `collection.toml` (or `.yaml`/`.json`) in each folder:
  ```toml
  description = "Street photography from Lagos, 2025"
  cover = "img_003.jpg"
  ```
- [ ] If no metadata file, use folder name as collection name and first image as cover
- [ ] Photo-level metadata: lean on server-side EXIF extraction rather than requiring per-photo config files — keep the workflow simple

### 5.7 — CLI Commands
- [ ] `esemese sync <directory>` — scan directory, diff against remote, upload new photos/collections
- [ ] `esemese sync <directory> --dry-run` — show what would happen without doing anything
- [ ] `esemese sync <directory> --apply-deletes` — also delete remote photos/collections that no longer exist locally
- [ ] `esemese status` — show current sync state (what's synced, what's pending)
- [ ] `esemese login` — authenticate and cache credentials

**Think about:**
- This CLI is just another client to your API. It uses the exact same endpoints as the browser admin UI. No special server-side support needed — that's the payoff of building a clean API.
- The state file (`.esemese-sync.json`) is important. Without it, the CLI has to re-diff every file against the API on every run. With it, syncs are fast because you know what's already been uploaded.
- You could eventually add a `--watch` flag that uses `notify` (Rust crate for filesystem events) to auto-sync on changes. But build the manual `sync` command first — the watcher is just a loop that calls sync.

---

## Build Order Summary

| # | What | Why first |
|---|------|-----------|
| 1 | Hetzner VPS + Postgres + RustFS (all on one box) | Everything depends on infra |
| 2 | Backend: config, DB migrations, auth | Foundation for all endpoints |
| 3 | Backend: presigned upload flow (initiate → PUT → confirm) | Core system design piece |
| 4 | Backend: public read endpoints (photos, collections, presigned download) | Needed for frontend |
| 5 | Frontend: scaffold TanStack Start, public gallery pages | Get something visible |
| 6 | Frontend: admin login + upload interface | Connect the upload flow |
| 7 | Backend: thumbnail generation | Needed for good gallery UX |
| 8 | Backend: CRUD for collections/photos management | Admin management |
| 9 | Frontend: admin management pages | Complete admin story |
| 10 | Deployment: Docker, Caddy, deploy to VPS | Go live |
| 11 | Polish: SEO, EXIF extraction, search, lightbox | Refinement |
| 12 | CLI: sync tool (`esemese sync ~/Photos/portfolio`) | Workflow automation, local backup |

---

## Key Design Decisions to Make Before You Start

- [x] ~~**Hetzner Object Storage vs self-hosted object storage?**~~ — Going with RustFS on the VPS (dev and prod). Escape hatches: swap to MinIO (same ports/API) if RustFS has bugs, or swap endpoint to Hetzner Object Storage if you outgrow the disk.
- [x] ~~**Thumbnails: public or presigned?**~~ — **Public derivatives behind CDN, presign only the private original** (decided 2026-06-08, see 1.5). Portfolio is read-heavy public content; presigning it would break caching.
- [x] ~~**Derivative sizes?**~~ — **Responsive ladder (400/800/1200/1600/2400) × AVIF+WebP, modelled as a `photo_variants` table**, generated **async** (see 1.7). Replaces the fixed `thumbnail_s3_key_small/_medium` columns.
- [ ] **Single bucket with prefixes vs multiple buckets?** — e.g. `originals/` (private) vs `variants/` (public). Prefixes are simplest; the visibility split is what matters, not the bucket count.
- [x] ~~**Original keys: content-addressed vs filename?**~~ — **`photos.s3_key = originals/{photo_id}` (UUID)** (decided 2026-06-08, see 1.4). Dedupe isn't needed at single-photographer scale; immutability is recovered on the variants by content-hashing them server-side. Browser never hashes.
- [ ] **Multipart upload for large files or just single PUT?** — Single PUT is sufficient for photos. Multipart is an optional Dropbox-like learning exercise.
- [x] ~~**EXIF extraction: client-side or server-side?**~~ — **Server-side** (`kamadak-exif`), during async processing. Never trust client metadata; strip GPS from public derivatives.
- [ ] **TanStack Start deployment: Node server or static export?** — You want SSR for SEO, so it'll be a Node server. Plan for that in your Docker setup.

///////////////////////////////////////////////////////////////////////////
ME: let's move to task 1.7, deverivative generation. As usual this is a learning journey for me. NO code generation or showing code. I want to walk the tough path and learn how to solve this myself from first principles. I'd say you shouldn't list our the tasks for me but I
dont even know where to start solving this. So your job is to guide me with the concepts, the whys and help me understand rust concets and idiomatic rust.

AI: 
Love this. Let me orient you without handing you a checklist — because the most important lesson here is what question to ask first, and most people ask the wrong one.

The trap to avoid

Your instinct will probably be "how do I resize an image in Rust?" — i.e. reach for a library. That's the wrong starting point. The image manipulation is the easy, well-trodden part. The part that actually determines whether your system is correct is a question that has
nothing to do with images:

▎ What process runs this work, when is it triggered, and what happens if it dies halfway through?

Get that wrong and you have photos stuck in processing forever, or work that silently vanishes on a restart. So start there.

The shape of the territory (a map, not a to-do list)

Whatever you build, the work has five conceptual moments: something triggers it → it fetches the original bytes → it transforms them (decode, resize ladder, encode, blurhash, EXIF) → it stores the results → it finalizes state (processing → ready, or → failed). Hold that
map loosely. The interesting decisions all live in the first and last moments — the triggering and the state transitions — not the middle.

The first real decision: the execution model

Reason through this design space yourself, but here's the lens to reason with — durability: "if the power cord is yanked mid-encode, what happens?"

- Synchronously inside confirm — you already rejected this, and you should understand why it's the baseline you're improving on: it couples a fast HTTP response to seconds of CPU, and ties up an actix worker. Reject it, but know what you're rejecting.
- Fire-and-forget background task spawned from confirm — trivially simple, in-process. Now apply the durability lens: the server restarts mid-encode… and that job is gone. The photo sits in processing with nothing coming to rescue it. Ask yourself: is that acceptable?
  What would recover it?
- A worker that polls the database for work — slower to react, more moving parts. But durability-wise it's a different universe: the record of "this needs work" lives in Postgres, so a crash just means it gets picked up next poll.

Here's the realization I want you to arrive at rather than be told: you may already have a queue. A photos row with status='processing' and no variants rows is, definitionally, a pending job. The database is the durable record. So "enqueue a job" might not mean a new
table or Redis — it might just mean "a loop that asks the DB what's unfinished." Sit with whether that's enough for you, or whether you want an explicit jobs table (the answer hinges on questions like: do you need retry counts? scheduled retries? visibility into
failures? — let those requirements drive the choice, don't pick the fancy option preemptively).

The reason this is the first decision: where the code lives, how it gets its database pool and S3 client, how it recovers, whether it can run more than one at a time — all of that hangs off this choice. Decide it and the rest has a frame.

The one Rust concept that will dominate everything

Independent of which execution model you pick, this will shape your code, so internalize it now: image decode/resize/encode is CPU-bound, synchronous, blocking work — and you must not run it directly on an async (Tokio) thread.

Why this matters, from first principles: Tokio runs your async tasks on a small pool of threads using cooperative scheduling — each task is expected to hit an .await and yield quickly so the thread can serve others. A 3-second image encode never yields; it monopolizes
that thread, and every other request/task assigned to it stalls. You'd see your whole server get janky whenever a photo processes. The idiomatic escape hatch is tokio::task::spawn_blocking (or handing CPU work to a dedicated pool like rayon): it moves blocking work onto
threads meant to block, keeping the async runtime responsive. Understanding why — the cooperative-scheduling model, the distinction between "waiting on I/O" (async's job) and "burning CPU" (blocking's job) — is one of the most important mental models in async Rust, and
this feature is the perfect place to learn it for real.

This also forces you to confront ownership/Send: to hand decoded image data to a blocking thread, that data has to move across the thread boundary, which means thinking about what owns the bytes and whether your types are Send. Good, idiomatic-Rust territory.

Where to actually start

Don't open the image crate docs yet. Start by deciding and sketching the execution model — on paper, in plain language:

1. What thing runs the work? (a task spawned at startup that loops? something else?)
2. How does it find work to do? (and given your "DB is the queue" option — what exact query represents "an unfinished photo"?)
3. How does it avoid grabbing the same photo twice if it ever runs concurrently, and what happens to a photo whose processing crashed — how does it get retried, and how do you stop a poison photo from retrying forever?
4. Where do decode/encode run so they don't block the runtime?

Answer those four in prose first. Once you've got a model you believe in, then we go deep on the image pipeline (the resize-ladder math, why you only ever downscale, the AVIF-encoding-is-expensive reality, blurhash, EXIF extraction) — and the Rust mechanics of moving
image data across spawn_blocking boundaries.

So: what's your instinct on #1 and #2 — what runs it, and how does it find work? Talk me through your reasoning and I'll push on it.