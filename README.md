# Oxipress 🦀

A fast Markdown blog template for Vercel, built entirely in Rust.

Inspired by [MDXpress-Blog-CNP](https://github.com/eshan-singh78/MDXpress-Blog-CNP),
but instead of compiling Markdown on every request in a Node function, Oxipress
pre-renders everything to static HTML at build time with a Rust binary — pages
are served straight from Vercel's CDN with zero cold starts. The only
serverless function is `/api/search`, a tiny stripped Rust binary.

## Features

- **Static output, zero client framework** — plain HTML + one small CSS file
- **Build-time syntax highlighting** (syntect) — no highlight.js shipped to readers
- **Full-text search** — Rust serverless function over a pre-built index
- **SEO pack** — RSS feed, `sitemap.xml`, Open Graph meta, canonical URLs
- **Tags** — frontmatter tags become `/tags/<tag>/` pages
- **Reading time**, dark mode (system preference + toggle), drafts
- GitHub-flavored Markdown: tables, footnotes, task lists, strikethrough, emoji shortcodes

## Quick start

```bash
# 1. Configure your site
$EDITOR blog.toml

# 2. Write a post
cp content/hello-world.md content/my-post.md

# 3. Build locally
cargo run -p generator            # add --drafts to include drafts
vercel dev                        # preview at http://localhost:3000 (runs /api/search too)

# 4. Deploy
vercel
```

## Writing posts

Drop a `.md` file in `content/`. The filename becomes the slug.

```yaml
---
title: My Post
date: 2026-07-01
tags: [rust, web]
description: Used in lists, search results, meta tags, and RSS.
draft: false        # optional; drafts are skipped in production builds
---
```

## How it works

| Path | What |
| ---- | ---- |
| `generator/` | Rust binary; renders `content/*.md` → `dist/` (pages, tags, RSS, sitemap, search index) |
| `api/search.rs` | Rust serverless function (`vercel-rust` runtime); scores posts against the embedded index |
| `assets/` | CSS + two tiny JS files (theme toggle, search widget), copied verbatim to `dist/` |
| `build.sh` | Vercel build command: installs Rust, runs the generator, syncs the search index |
| `blog.toml` | Site title, description, base URL, author |

`api/search_index.json` is generated — `build.sh` refreshes it on every deploy,
and a committed copy keeps the function compilable. Set `base_url` in
`blog.toml` to your production domain so RSS/sitemap/OG URLs are correct.

## Development

```bash
cargo test
cargo run -p generator && cp dist/search-index.json api/search_index.json
vercel dev 
```

## License

MIT
