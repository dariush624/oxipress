---
title: Why Rust on Vercel?
date: 2026-07-02
tags: [rust, vercel, performance]
description: How pre-rendering with Rust beats per-request Markdown compilation.
---

Most minimal Markdown blog templates for Vercel do the same thing: a Node
serverless function reads a `.md` file, compiles it, and returns HTML — on
*every single request*. That means cold starts, repeated parsing, and no CDN
caching by default.

Oxipress flips the model:

1. At **build time**, a Rust binary compiles all Markdown once — with syntax
   highlighting, tag pages, RSS, and a sitemap.
2. The output is plain static files, served from the edge with zero latency
   and zero function invocations.
3. The only serverless function is `/api/search` — a Rust binary that scores
   posts against a pre-built index in microseconds.

The result: page loads are CDN-fast, the free tier goes much further, and the
one function you do pay for is a tiny stripped Rust binary instead of a Node
runtime.
