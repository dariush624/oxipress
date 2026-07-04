---
title: Hello, World
date: 2026-07-01
tags: [meta, rust]
description: Welcome to Oxipress — a Rust-powered Markdown blog template for Vercel.
---

Welcome to **Oxipress**! This entire site was pre-rendered by a Rust binary at
build time, so every page you see is static HTML served straight from Vercel's
CDN. No server, no database, no client-side framework.

## Writing posts

Drop a Markdown file into `content/`, push, done. Each post needs a small
frontmatter block:

```yaml
---
title: My Post
date: 2026-07-01
tags: [rust, web]
description: Shows up in lists, search results, and meta tags.
draft: false
---
```

## What you get for free

- Syntax highlighting at build time (zero client JS):

```rust
fn main() {
    let posts = load_posts("content/")?;
    println!("rendered {} posts", posts.len());
}
```

- GitHub-flavored tables, footnotes[^1], ~~strikethrough~~, and task lists:

| Feature | Original template | Oxipress |
| ------- | ----------------- | -------- |
| Rendering | Per-request (Node) | Build time (Rust) |
| RSS + sitemap | ✗ | ✓ |
| Search | ✗ | ✓ (Rust serverless fn) |

- [ ] A task list item
- [x] A completed one

[^1]: Footnotes work too.
