use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::json;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

static INDEX_JSON: &str = include_str!("search_index.json");

#[derive(Deserialize, Serialize)]
struct Entry {
    slug: String,
    title: String,
    description: String,
    tags: Vec<String>,
    body: String,
}

static INDEX: LazyLock<Vec<Entry>> =
    LazyLock::new(|| serde_json::from_str(INDEX_JSON).expect("valid search index"));

#[tokio::main]
async fn main() -> Result<(), Error> {
    run(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    let url = url::Url::parse(&req.uri().to_string())?;
    let query = url
        .query_pairs()
        .find(|(k, _)| k == "q")
        .map(|(_, v)| v.to_lowercase())
        .unwrap_or_default();

    if query.trim().is_empty() {
        return json_response(StatusCode::BAD_REQUEST, json!({ "error": "missing q" }));
    }

    let terms: Vec<&str> = query.split_whitespace().collect();
    let mut scored: Vec<(u32, &Entry)> = INDEX
        .iter()
        .filter_map(|e| {
            let s = score(e, &terms);
            (s > 0).then_some((s, e))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));

    let results: Vec<_> = scored
        .iter()
        .take(10)
        .map(|(_, e)| json!({ "slug": e.slug, "title": e.title, "description": e.description }))
        .collect();
    json_response(StatusCode::OK, json!({ "results": results }))
}

fn score(entry: &Entry, terms: &[&str]) -> u32 {
    let title = entry.title.to_lowercase();
    let desc = entry.description.to_lowercase();
    let body = entry.body.to_lowercase();
    let mut score = 0;
    for term in terms {
        if title.contains(term) {
            score += 8;
        }
        if entry.tags.iter().any(|t| t.to_lowercase() == *term) {
            score += 5;
        }
        if desc.contains(term) {
            score += 3;
        }
        score += body.matches(term).count().min(5) as u32;
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_parses_and_scores() {
        assert!(!INDEX.is_empty(), "committed search_index.json is empty");
        let terms = vec!["rust"];
        let hits: Vec<_> = INDEX.iter().filter(|e| score(e, &terms) > 0).collect();
        assert!(!hits.is_empty(), "expected sample posts to match 'rust'");
    }

    #[test]
    fn title_match_outscores_body_match() {
        let title_hit = Entry {
            slug: "a".into(),
            title: "Rust rules".into(),
            description: String::new(),
            tags: vec![],
            body: String::new(),
        };
        let body_hit = Entry {
            slug: "b".into(),
            title: "Other".into(),
            description: String::new(),
            tags: vec![],
            body: "rust".into(),
        };
        assert!(score(&title_hit, &["rust"]) > score(&body_hit, &["rust"]));
    }
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Result<Response<Body>, Error> {
    Ok(Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "s-maxage=60, stale-while-revalidate=300")
        .body(body.to_string().into())?)
}
