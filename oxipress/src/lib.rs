mod content;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use comrak::options::Plugins;
use comrak::plugins::syntect::SyntectAdapter;
use comrak::{markdown_to_html_with_plugins, Options};
use content::{escape_xml, plain_text, reading_time_min, rfc2822_date, split_frontmatter, Post};
use minijinja::{context, Environment};
use serde::Deserialize;

const ASSETS: &[(&str, &str)] = &[
    ("style.css", include_str!("../assets/style.css")),
    ("theme.js", include_str!("../assets/theme.js")),
    ("search.js", include_str!("../assets/search.js")),
    ("favicon.svg", include_str!("../assets/favicon.svg")),
];

#[derive(Debug, Deserialize)]
struct Config {
    title: String,
    description: String,
    base_url: String,
    author: String,
    #[serde(default = "default_language")]
    language: String,
    #[serde(default)]
    image: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    linkedin: Option<String>,
    #[serde(default)]
    allow_ai_training: bool,
}

const AI_SEARCH_BOTS: &[&str] = &[
    "OAI-SearchBot",
    "ChatGPT-User",
    "Claude-SearchBot",
    "Claude-User",
    "PerplexityBot",
    "Perplexity-User",
    "DuckAssistBot",
    "MistralAI-User",
];

const AI_TRAINING_BOTS: &[&str] = &[
    "GPTBot",
    "ClaudeBot",
    "anthropic-ai",
    "Google-Extended",
    "Applebot-Extended",
    "CCBot",
    "meta-externalagent",
    "FacebookBot",
    "Bytespider",
    "Amazonbot",
    "cohere-ai",
    "cohere-training-data-crawler",
    "AI2Bot",
    "Ai2Bot-Dolma",
    "Diffbot",
    "omgili",
    "omgilibot",
    "Webzio-Extended",
    "ImagesiftBot",
    "PanguBot",
    "Kangaroo Bot",
    "img2dataset",
];

fn default_language() -> String {
    "en".to_string()
}

pub fn project_root() -> PathBuf {
    let cwd = std::env::current_dir().expect("cwd");
    if cwd.join("blog.toml").exists() {
        cwd
    } else {
        cwd.parent().map(Path::to_path_buf).unwrap_or(cwd)
    }
}

pub fn build(root: impl AsRef<Path>, include_drafts: bool) -> Result<(), String> {
    let root = root.as_ref();
    let config: Config = toml::from_str(
        &fs::read_to_string(root.join("blog.toml")).map_err(|e| format!("blog.toml: {e}"))?,
    )
    .map_err(|e| format!("blog.toml: {e}"))?;
    let base_url = config.base_url.trim_end_matches('/').to_string();
    let site_image = config.image.as_deref().map(|i| absolute_url(&base_url, i));

    let posts = load_posts(&root.join("content"), include_drafts)?;
    let out = root.join("dist");
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).map_err(|e| e.to_string())?;

    let mut env = Environment::new();
    env.add_template("base.html", include_str!("../templates/base.html"))
        .map_err(|e| e.to_string())?;
    env.add_template("index.html", include_str!("../templates/index.html"))
        .map_err(|e| e.to_string())?;
    env.add_template("post.html", include_str!("../templates/post.html"))
        .map_err(|e| e.to_string())?;
    env.add_template("tag.html", include_str!("../templates/tag.html"))
        .map_err(|e| e.to_string())?;
    env.add_template("404.html", include_str!("../templates/404.html"))
        .map_err(|e| e.to_string())?;

    let site = context! {
        title => config.title,
        description => config.description,
        base_url => base_url,
        author => config.author,
        language => config.language,
        email => config.email,
        linkedin => config.linkedin,
    };

    let post_ctx = |p: &Post| {
        context! {
            slug => p.slug,
            title => p.title,
            date => p.date.to_string(),
            updated => p.updated.map(|d| d.to_string()),
            tags => p.tags,
            description => p.description,
            reading_time => p.reading_time_min,
            html => p.html,
        }
    };

    let post_list: Vec<_> = posts.iter().map(post_ctx).collect();
    let html = env
        .get_template("index.html")
        .unwrap()
        .render(context! {
            site => site,
            posts => post_list,
            page_url => format!("{base_url}/"),
            social_image => site_image,
            json_ld => website_json_ld(&config, &base_url),
        })
        .map_err(|e| e.to_string())?;
    write(&out.join("index.html"), &html)?;

    for p in &posts {
        let image = p
            .image
            .as_deref()
            .map(|i| absolute_url(&base_url, i))
            .or_else(|| site_image.clone());
        let html = env
            .get_template("post.html")
            .unwrap()
            .render(context! {
                site => site,
                post => post_ctx(p),
                page_url => format!("{base_url}/posts/{}/", p.slug),
                social_image => image,
                json_ld => post_json_ld(&config, &base_url, p, image.as_deref()),
            })
            .map_err(|e| e.to_string())?;
        write(&out.join("posts").join(&p.slug).join("index.html"), &html)?;
    }

    let mut tags: BTreeMap<&str, Vec<&Post>> = BTreeMap::new();
    for p in &posts {
        for t in &p.tags {
            tags.entry(t).or_default().push(p);
        }
    }
    for (tag, tagged) in &tags {
        let list: Vec<_> = tagged.iter().map(|p| post_ctx(p)).collect();
        let html = env
            .get_template("tag.html")
            .unwrap()
            .render(context! {
                site => site,
                tag => tag,
                posts => list,
                page_url => format!("{base_url}/tags/{tag}/"),
                social_image => site_image,
                json_ld => tag_json_ld(&config, &base_url, tag, tagged),
            })
            .map_err(|e| e.to_string())?;
        write(&out.join("tags").join(tag).join("index.html"), &html)?;
    }

    let html = env
        .get_template("404.html")
        .unwrap()
        .render(context! { site => site })
        .map_err(|e| e.to_string())?;
    write(&out.join("404.html"), &html)?;

    write_feed(&out, &config, &base_url, &posts)?;
    write_sitemap(&out, &base_url, &posts, &tags)?;
    write_robots(&out, &config, &base_url)?;
    write_llms_txt(&out, &config, &base_url, &posts)?;
    write_humans_txt(&out, &config, &posts)?;
    write_search_index(&out, &posts)?;
    write_assets(&out)?;
    copy_dir(&root.join("static"), &out)?;

    println!(
        "built {} posts, {} tags -> {}",
        posts.len(),
        tags.len(),
        out.display()
    );
    Ok(())
}

fn load_posts(content_dir: &Path, include_drafts: bool) -> Result<Vec<Post>, String> {
    let adapter = SyntectAdapter::new(Some("base16-ocean.dark"));
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.strikethrough = true;
    options.extension.autolink = true;
    options.extension.footnotes = true;
    options.extension.tasklist = true;
    options.extension.header_id_prefix = Some(String::new());
    options.extension.shortcodes = true;
    let mut plugins = Plugins::default();
    plugins.render.codefence_syntax_highlighter = Some(&adapter);

    let mut posts = Vec::new();
    let entries =
        fs::read_dir(content_dir).map_err(|e| format!("{}: {e}", content_dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let slug = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("bad filename: {}", path.display()))?
            .to_string();
        let source = fs::read_to_string(&path).map_err(|e| format!("{slug}: {e}"))?;
        let (fm, body) = split_frontmatter(&source).map_err(|e| format!("{slug}: {e}"))?;
        if fm.draft && !include_drafts {
            continue;
        }
        let plain = plain_text(body);
        let reading_time_min = reading_time_min(&plain);
        posts.push(Post {
            slug,
            title: fm.title,
            date: fm.date,
            updated: fm.updated,
            tags: fm.tags,
            description: fm.description,
            image: fm.image,
            html: markdown_to_html_with_plugins(body, &options, &plugins),
            reading_time_min,
        });
    }
    posts.sort_by(|a, b| b.date.cmp(&a.date).then(a.slug.cmp(&b.slug)));
    Ok(posts)
}

fn absolute_url(base_url: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else {
        format!("{base_url}/{}", path.trim_start_matches('/'))
    }
}

fn latest_update<'a>(posts: impl IntoIterator<Item = &'a Post>) -> Option<NaiveDate> {
    posts.into_iter().map(Post::last_modified).max()
}

fn write_feed(out: &Path, config: &Config, base_url: &str, posts: &[Post]) -> Result<(), String> {
    let mut items = String::new();
    for p in posts.iter().take(20) {
        let url = format!("{base_url}/posts/{}/", p.slug);
        let categories: String = p
            .tags
            .iter()
            .map(|t| format!("    <category>{}</category>\n", escape_xml(t)))
            .collect();
        items.push_str(&format!(
            "  <item>\n    <title>{}</title>\n    <link>{url}</link>\n    <guid>{url}</guid>\n    <pubDate>{}</pubDate>\n    <description>{}</description>\n{categories}  </item>\n",
            escape_xml(&p.title),
            rfc2822_date(&p.date),
            escape_xml(&p.description),
        ));
    }
    let last_build = latest_update(posts)
        .map(|d| format!("  <lastBuildDate>{}</lastBuildDate>\n", rfc2822_date(&d)))
        .unwrap_or_default();
    let feed = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\"><channel>\n  <title>{}</title>\n  <link>{base_url}/</link>\n  <description>{}</description>\n  <language>{}</language>\n  <atom:link href=\"{base_url}/feed.xml\" rel=\"self\" type=\"application/rss+xml\"/>\n{last_build}{items}</channel></rss>\n",
        escape_xml(&config.title),
        escape_xml(&config.description),
        escape_xml(&config.language),
    );
    write(&out.join("feed.xml"), &feed)
}

fn write_sitemap(
    out: &Path,
    base_url: &str,
    posts: &[Post],
    tags: &BTreeMap<&str, Vec<&Post>>,
) -> Result<(), String> {
    let mut urls = vec![(format!("{base_url}/"), latest_update(posts))];
    urls.extend(
        posts
            .iter()
            .map(|p| (format!("{base_url}/posts/{}/", p.slug), Some(p.last_modified()))),
    );
    urls.extend(
        tags.iter()
            .map(|(t, tagged)| (format!("{base_url}/tags/{t}/"), latest_update(tagged.iter().copied()))),
    );
    let body: String = urls
        .iter()
        .map(|(u, lastmod)| match lastmod {
            Some(d) => format!("  <url><loc>{}</loc><lastmod>{d}</lastmod></url>\n", escape_xml(u)),
            None => format!("  <url><loc>{}</loc></url>\n", escape_xml(u)),
        })
        .collect();
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{body}</urlset>\n"
    );
    write(&out.join("sitemap.xml"), &xml)
}

fn write_robots(out: &Path, config: &Config, base_url: &str) -> Result<(), String> {
    let group = |bots: &[&str], rule: &str| {
        let agents: String = bots.iter().map(|b| format!("User-agent: {b}\n")).collect();
        format!("{agents}{rule}\n\n")
    };
    let mut txt = String::from("User-agent: *\nAllow: /\n\n");
    txt.push_str(&group(AI_SEARCH_BOTS, "Allow: /"));
    if !config.allow_ai_training {
        txt.push_str(&group(AI_TRAINING_BOTS, "Disallow: /"));
    }
    txt.push_str(&format!("Sitemap: {base_url}/sitemap.xml\n"));
    write(&out.join("robots.txt"), &txt)
}

fn write_llms_txt(out: &Path, config: &Config, base_url: &str, posts: &[Post]) -> Result<(), String> {
    let mut about = format!("Blog by {}.", config.author);
    if let Some(linkedin) = &config.linkedin {
        about.push_str(&format!(" LinkedIn: {linkedin}."));
    }
    if let Some(email) = &config.email {
        about.push_str(&format!(" Contact: {email}."));
    }
    let mut index = format!(
        "# {}\n\n> {}\n\n{about}\n\n## Posts\n\n",
        config.title, config.description
    );
    for p in posts {
        let url = format!("{base_url}/posts/{}/", p.slug);
        if p.description.is_empty() {
            index.push_str(&format!("- [{}]({url})\n", p.title));
        } else {
            index.push_str(&format!("- [{}]({url}): {}\n", p.title, p.description));
        }
    }
    index.push_str(&format!(
        "\n## Optional\n\n- [RSS feed]({base_url}/feed.xml)\n- [Sitemap]({base_url}/sitemap.xml)\n"
    ));
    write(&out.join("llms.txt"), &index)
}

fn write_humans_txt(out: &Path, config: &Config, posts: &[Post]) -> Result<(), String> {
    let mut txt = format!("/* TEAM */\nAuthor: {}\n", config.author);
    if let Some(email) = &config.email {
        txt.push_str(&format!("Contact: {email}\n"));
    }
    if let Some(linkedin) = &config.linkedin {
        txt.push_str(&format!("LinkedIn: {linkedin}\n"));
    }
    txt.push_str("\n/* SITE */\n");
    if let Some(d) = latest_update(posts) {
        txt.push_str(&format!("Last update: {}\n", d.format("%Y/%m/%d")));
    }
    txt.push_str(&format!(
        "Language: {}\nStandards: HTML5, CSS3, RSS 2.0, schema.org\nSoftware: Oxipress (Rust)\n",
        config.language
    ));
    write(&out.join("humans.txt"), &txt)
}

fn person_json_ld(config: &Config, base_url: &str) -> serde_json::Value {
    let mut person = serde_json::json!({
        "@type": "Person",
        "name": config.author,
        "url": format!("{base_url}/"),
    });
    if let Some(email) = &config.email {
        person["email"] = email.as_str().into();
    }
    if let Some(linkedin) = &config.linkedin {
        person["sameAs"] = serde_json::json!([linkedin]);
    }
    person
}

fn breadcrumb_json_ld(trail: &[(&str, String)]) -> serde_json::Value {
    let items: Vec<_> = trail
        .iter()
        .enumerate()
        .map(|(i, (name, url))| {
            serde_json::json!({
                "@type": "ListItem",
                "position": i + 1,
                "name": name,
                "item": url,
            })
        })
        .collect();
    serde_json::json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": items,
    })
}

fn website_json_ld(config: &Config, base_url: &str) -> String {
    serde_json::json!({
        "@context": "https://schema.org",
        "@type": ["WebSite", "Blog"],
        "url": format!("{base_url}/"),
        "name": config.title,
        "description": config.description,
        "inLanguage": config.language,
        "author": person_json_ld(config, base_url),
        "publisher": person_json_ld(config, base_url),
    })
    .to_string()
}

fn post_json_ld(config: &Config, base_url: &str, p: &Post, image: Option<&str>) -> String {
    let url = format!("{base_url}/posts/{}/", p.slug);
    let mut posting = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "BlogPosting",
        "headline": p.title,
        "description": p.description,
        "url": url,
        "mainEntityOfPage": { "@type": "WebPage", "@id": url },
        "datePublished": p.date.to_string(),
        "dateModified": p.last_modified().to_string(),
        "inLanguage": config.language,
        "keywords": p.tags,
        "author": person_json_ld(config, base_url),
        "publisher": person_json_ld(config, base_url),
        "isPartOf": {
            "@type": "Blog",
            "@id": format!("{base_url}/"),
            "name": config.title,
        },
    });
    if let Some(image) = image {
        posting["image"] = image.into();
    }
    let breadcrumb = breadcrumb_json_ld(&[
        (config.title.as_str(), format!("{base_url}/")),
        (p.title.as_str(), url),
    ]);
    serde_json::json!([posting, breadcrumb]).to_string()
}

fn tag_json_ld(config: &Config, base_url: &str, tag: &str, tagged: &[&Post]) -> String {
    let url = format!("{base_url}/tags/{tag}/");
    let name = format!("#{tag}");
    let items: Vec<_> = tagged
        .iter()
        .enumerate()
        .map(|(i, p)| {
            serde_json::json!({
                "@type": "ListItem",
                "position": i + 1,
                "url": format!("{base_url}/posts/{}/", p.slug),
                "name": p.title,
            })
        })
        .collect();
    let page = serde_json::json!({
        "@context": "https://schema.org",
        "@type": "CollectionPage",
        "url": url,
        "name": name,
        "inLanguage": config.language,
        "isPartOf": {
            "@type": "WebSite",
            "@id": format!("{base_url}/"),
            "name": config.title,
        },
        "mainEntity": { "@type": "ItemList", "itemListElement": items },
    });
    let breadcrumb = breadcrumb_json_ld(&[
        (config.title.as_str(), format!("{base_url}/")),
        (name.as_str(), url),
    ]);
    serde_json::json!([page, breadcrumb]).to_string()
}

fn write_search_index(out: &Path, posts: &[Post]) -> Result<(), String> {
    let index: Vec<_> = posts
        .iter()
        .map(|p| {
            serde_json::json!({
                "slug": p.slug,
                "title": p.title,
                "description": p.description,
                "tags": p.tags,
            })
        })
        .collect();
    let json = serde_json::to_string(&index).map_err(|e| e.to_string())?;
    write(&out.join("search-index.json"), &json)
}

fn write_assets(out: &Path) -> Result<(), String> {
    for (name, contents) in ASSETS {
        write(&out.join(name), contents)?;
    }
    Ok(())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dst).map_err(|e| format!("{}: {e}", dst.display()))?;
    for entry in fs::read_dir(src).map_err(|e| format!("{}: {e}", src.display()))? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let target = dst.join(path.file_name().expect("dir entry has a name"));
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

fn write(path: &Path, contents: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(path, contents).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_seo_files() {
        let root = std::env::temp_dir().join(format!("oxipress-seo-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("content")).unwrap();
        fs::create_dir_all(root.join("static")).unwrap();
        fs::write(
            root.join("blog.toml"),
            "title = \"T\"\ndescription = \"D\"\nbase_url = \"https://ex.com/\"\nauthor = \"A\"\nimage = \"/og.png\"\nlinkedin = \"https://www.linkedin.com/in/a/\"\n",
        )
        .unwrap();
        fs::write(
            root.join("content/p.md"),
            "---\ntitle: P\ndate: 2026-01-02\nupdated: 2026-02-03\ntags: [rust]\ndescription: Desc\n---\n\nBody.\n",
        )
        .unwrap();
        fs::write(root.join("static/og.png"), "png").unwrap();

        build(&root, false).unwrap();
        let read = |p: &str| fs::read_to_string(root.join("dist").join(p)).unwrap();

        let robots = read("robots.txt");
        assert!(robots.contains("User-agent: *\nAllow: /\n"));
        assert!(robots.contains("User-agent: OAI-SearchBot\n"));
        assert!(robots.contains("User-agent: img2dataset\nDisallow: /\n"));
        assert!(robots.contains("Sitemap: https://ex.com/sitemap.xml"));
        assert!(read("sitemap.xml")
            .contains("<loc>https://ex.com/posts/p/</loc><lastmod>2026-02-03</lastmod>"));
        assert!(read("feed.xml").contains("<atom:link href=\"https://ex.com/feed.xml\""));
        assert!(read("llms.txt").contains("- [P](https://ex.com/posts/p/): Desc"));
        assert!(!root.join("dist/llms-full.txt").exists());
        assert!(read("humans.txt").contains("LinkedIn: https://www.linkedin.com/in/a/"));
        assert_eq!(read("og.png"), "png");
        assert!(read("404.html").contains("noindex"));
        assert!(read("index.html").contains("<h1 class=\"site-title\">"));

        let post = read("posts/p/index.html");
        assert!(post.contains("og:image"));
        assert!(post.contains("summary_large_image"));
        assert!(post.contains("article:modified_time"));
        assert!(post.contains("\"dateModified\":\"2026-02-03\""));
        assert!(post.contains("BreadcrumbList"));

        fs::remove_dir_all(&root).unwrap();
    }
}
