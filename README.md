# Oxipress 🦀

A fast Markdown blog, built entirely in Rust. Pre-renders everything to static
HTML at build time — pages are served straight from a CDN with zero cold starts
and **no serverless functions at all**. Deploys as-is to Cloudflare Pages,
GitHub Pages, Netlify, or any static host.

## Features

- **Static output, zero client framework** — plain HTML + one small CSS file
- **Build-time syntax highlighting** (syntect) — no highlight.js shipped to readers
- **Full static search** — a lean JSON index (title / description / tags, no post
  bodies) is fetched once and ranked client-side; a few hundred bytes per post,
  no backend
- **SEO pack** — RSS feed, `sitemap.xml` with `lastmod`, canonical URLs,
  `robots.txt` that allows search (incl. AI search like OAI-SearchBot / PerplexityBot)
  but blocks AI training crawlers by default (`allow_ai_training = true` to opt in),
  Open Graph / Twitter cards (with `og:image`), schema.org JSON-LD (`BlogPosting` /
  `WebSite` / `CollectionPage` / `BreadcrumbList`), `404.html`, favicon
- **AI & humans** — `llms.txt` (llmstxt.org) and `humans.txt`
- **Tags** — frontmatter tags become `/tags/<tag>/` pages
- **Reading time**, dark mode (system preference + toggle), drafts
- GitHub-flavored Markdown: tables, footnotes, task lists, strikethrough, emoji shortcodes

## Use it as a crate

The renderer is the `oxipress` library crate. Your blog repo needs only
`blog.toml` and `content/*.md` — templates and assets are compiled into the
crate. In a separate blog repo:

```toml
# Cargo.toml
[dependencies]
oxipress = { git = "https://github.com/dariush624/oxipress" }
```

```rust
// src/main.rs — your blog's build binary
fn main() {
    let drafts = std::env::args().any(|a| a == "--drafts");
    oxipress::build(oxipress::project_root(), drafts).unwrap();
}
```

Or skip the wrapper entirely and install the bundled binary:

```bash
cargo install --git https://github.com/dariush624/oxipress oxipress
oxipress            # run from a dir containing blog.toml; writes ./dist
```

## Quick start (this repo)

```bash
$EDITOR blog.toml                 # 1. configure your site
cp content/hello-world.md content/my-post.md   # 2. write a post
cargo run -p oxipress             # 3. build -> dist/  (add --drafts for drafts)
python3 -m http.server -d dist    # 4. preview at http://localhost:8000
```

## Writing posts

Drop a `.md` file in `content/`. The filename becomes the slug.

```yaml
---
title: My Post
date: 2026-07-01
tags: [rust, web]
description: Used in lists, search results, meta tags, and RSS.
updated: 2026-07-15 # optional; drives dateModified, sitemap lastmod
image: /img/my-post.png  # optional; social preview, overrides blog.toml `image`
draft: false        # optional; drafts are skipped in production builds
---
```

## Deploy to Cloudflare Pages

- **Build command:** `bash build.sh` (installs Rust if missing, runs the generator)
- **Build output directory:** `dist`
- Set `base_url` in `blog.toml` to your production domain so RSS/sitemap/OG URLs are correct.

## How it works

| Path | What |
| ---- | ---- |
| `oxipress/` | The `oxipress` crate — library + `oxipress` binary; renders `content/*.md` → `dist/` (pages, tags, RSS, sitemap, search index) |
| `oxipress/templates/` | minijinja templates, compiled in |
| `oxipress/assets/` | CSS + two tiny JS files (theme toggle, static search), compiled in and emitted to `dist/` |
| `content/` | your Markdown posts |
| `static/` | optional; copied verbatim into `dist/` (images, `og.png`, custom `favicon.svg`, verification files) — overrides bundled files |
| `build.sh` | Cloudflare Pages build command |
| `blog.toml` | site title, description, base URL, author, optional `language`, `image`, `email` / `linkedin` (footer links + JSON-LD) |

## Development

```bash
cargo test
cargo run -p oxipress
```

## License

MIT
