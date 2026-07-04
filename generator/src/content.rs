use chrono::NaiveDate;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Frontmatter {
    pub title: String,
    pub date: NaiveDate,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub draft: bool,
}

#[derive(Debug)]
pub struct Post {
    pub slug: String,
    pub title: String,
    pub date: NaiveDate,
    pub tags: Vec<String>,
    pub description: String,
    pub html: String,
    pub plain_text: String,
    pub reading_time_min: u32,
}

pub fn split_frontmatter(source: &str) -> Result<(Frontmatter, &str), String> {
    let source = source.strip_prefix('\u{feff}').unwrap_or(source);
    let rest = source
        .strip_prefix("---")
        .and_then(|r| r.strip_prefix('\n').or_else(|| r.strip_prefix("\r\n")))
        .ok_or("missing frontmatter: file must start with ---")?;
    let end = rest
        .find("\n---")
        .ok_or("unterminated frontmatter: closing --- not found")?;
    let (yaml, body) = rest.split_at(end);
    let body = body
        .trim_start_matches('\n')
        .trim_start_matches("---")
        .trim_start_matches('\r')
        .trim_start_matches('\n');
    let fm: Frontmatter =
        serde_yaml::from_str(yaml).map_err(|e| format!("invalid frontmatter: {e}"))?;
    Ok((fm, body))
}

pub fn plain_text(markdown: &str) -> String {
    let mut out = String::with_capacity(markdown.len());
    let mut in_fence = false;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let cleaned: String = line
            .chars()
            .filter(|c| !matches!(c, '#' | '*' | '_' | '`' | '>' | '[' | ']' | '(' | ')'))
            .collect();
        let cleaned = cleaned.trim();
        if !cleaned.is_empty() {
            out.push_str(cleaned);
            out.push(' ');
        }
    }
    out.trim_end().to_string()
}

pub fn reading_time_min(plain: &str) -> u32 {
    let words = plain.split_whitespace().count() as u32;
    (words / 220).max(1)
}

pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub fn rfc2822_date(date: &NaiveDate) -> String {
    date.format("%a, %d %b %Y 00:00:00 GMT").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "---\ntitle: Hello\ndate: 2026-01-02\ntags: [rust, web]\ndescription: A post\n---\n\n# Heading\n\nBody text.\n";

    #[test]
    fn parses_frontmatter_and_body() {
        let (fm, body) = split_frontmatter(SAMPLE).unwrap();
        assert_eq!(fm.title, "Hello");
        assert_eq!(fm.date.to_string(), "2026-01-02");
        assert_eq!(fm.tags, vec!["rust", "web"]);
        assert!(!fm.draft);
        assert!(body.starts_with("# Heading"));
    }

    #[test]
    fn rejects_missing_frontmatter() {
        assert!(split_frontmatter("# no frontmatter\n").is_err());
    }

    #[test]
    fn plain_text_skips_code_fences() {
        let md = "Intro\n\n```rust\nlet x = 1;\n```\n\nOutro";
        let p = plain_text(md);
        assert!(p.contains("Intro"));
        assert!(p.contains("Outro"));
        assert!(!p.contains("let x"));
    }

    #[test]
    fn reading_time_has_floor_of_one() {
        assert_eq!(reading_time_min("few words"), 1);
        let long = "word ".repeat(500);
        assert_eq!(reading_time_min(&long), 2);
    }

    #[test]
    fn escapes_xml_entities() {
        assert_eq!(escape_xml("a<b & \"c\""), "a&lt;b &amp; &quot;c&quot;");
    }

    #[test]
    fn formats_rfc2822() {
        let d = NaiveDate::from_ymd_opt(2026, 1, 2).unwrap();
        assert_eq!(rfc2822_date(&d), "Fri, 02 Jan 2026 00:00:00 GMT");
    }
}
