mod content;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use comrak::plugins::syntect::SyntectAdapter;
use comrak::options::Plugins;
use comrak::{markdown_to_html_with_plugins, Options};
use content::{escape_xml, plain_text, reading_time_min, rfc2822_date, split_frontmatter, Post};
use minijinja::{context, Environment};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    title: String,
    description: String,
    base_url: String,
    author: String,
}

fn main() {
    let include_drafts = std::env::args().any(|a| a == "--drafts");
    if let Err(e) = build(include_drafts) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn build(include_drafts: bool) -> Result<(), String> {
    let root = project_root();
    let config: Config = toml::from_str(
        &fs::read_to_string(root.join("blog.toml")).map_err(|e| format!("blog.toml: {e}"))?,
    )
    .map_err(|e| format!("blog.toml: {e}"))?;
    let base_url = config.base_url.trim_end_matches('/').to_string();

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

    let site = context! {
        title => config.title,
        description => config.description,
        base_url => base_url,
        author => config.author,
    };

    let post_ctx = |p: &Post| {
        context! {
            slug => p.slug,
            title => p.title,
            date => p.date.to_string(),
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
        .render(context! { site => site, posts => post_list, page_url => format!("{base_url}/") })
        .map_err(|e| e.to_string())?;
    write(&out.join("index.html"), &html)?;

    for p in &posts {
        let html = env
            .get_template("post.html")
            .unwrap()
            .render(context! {
                site => site,
                post => post_ctx(p),
                page_url => format!("{base_url}/posts/{}/", p.slug),
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
            })
            .map_err(|e| e.to_string())?;
        write(&out.join("tags").join(tag).join("index.html"), &html)?;
    }

    write_feed(&out, &config, &base_url, &posts)?;
    write_sitemap(&out, &base_url, &posts, &tags)?;
    write_search_index(&out, &posts)?;
    copy_assets(&root.join("assets"), &out)?;

    println!(
        "built {} posts, {} tags -> {}",
        posts.len(),
        tags.len(),
        out.display()
    );
    Ok(())
}

fn project_root() -> PathBuf {
    let cwd = std::env::current_dir().expect("cwd");
    if cwd.join("blog.toml").exists() {
        cwd
    } else {
        cwd.parent().map(Path::to_path_buf).unwrap_or(cwd)
    }
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
    let entries = fs::read_dir(content_dir)
        .map_err(|e| format!("{}: {e}", content_dir.display()))?;
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
        posts.push(Post {
            slug,
            title: fm.title,
            date: fm.date,
            tags: fm.tags,
            description: fm.description,
            html: markdown_to_html_with_plugins(body, &options, &plugins),
            reading_time_min: reading_time_min(&plain),
            plain_text: plain,
        });
    }
    posts.sort_by(|a, b| b.date.cmp(&a.date).then(a.slug.cmp(&b.slug)));
    Ok(posts)
}

fn write_feed(out: &Path, config: &Config, base_url: &str, posts: &[Post]) -> Result<(), String> {
    let mut items = String::new();
    for p in posts.iter().take(20) {
        let url = format!("{base_url}/posts/{}/", p.slug);
        items.push_str(&format!(
            "  <item>\n    <title>{}</title>\n    <link>{url}</link>\n    <guid>{url}</guid>\n    <pubDate>{}</pubDate>\n    <description>{}</description>\n  </item>\n",
            escape_xml(&p.title),
            rfc2822_date(&p.date),
            escape_xml(&p.description),
        ));
    }
    let feed = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\"><channel>\n  <title>{}</title>\n  <link>{base_url}/</link>\n  <description>{}</description>\n{items}</channel></rss>\n",
        escape_xml(&config.title),
        escape_xml(&config.description),
    );
    write(&out.join("feed.xml"), &feed)
}

fn write_sitemap(
    out: &Path,
    base_url: &str,
    posts: &[Post],
    tags: &BTreeMap<&str, Vec<&Post>>,
) -> Result<(), String> {
    let mut urls = vec![format!("{base_url}/")];
    urls.extend(posts.iter().map(|p| format!("{base_url}/posts/{}/", p.slug)));
    urls.extend(tags.keys().map(|t| format!("{base_url}/tags/{t}/")));
    let body: String = urls
        .iter()
        .map(|u| format!("  <url><loc>{}</loc></url>\n", escape_xml(u)))
        .collect();
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{body}</urlset>\n"
    );
    write(&out.join("sitemap.xml"), &xml)
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
                "body": p.plain_text.chars().take(5000).collect::<String>(),
            })
        })
        .collect();
    let json = serde_json::to_string(&index).map_err(|e| e.to_string())?;
    write(&out.join("search-index.json"), &json)
}

fn copy_assets(assets: &Path, out: &Path) -> Result<(), String> {
    let entries = fs::read_dir(assets).map_err(|e| format!("{}: {e}", assets.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_file() {
            let dest = out.join(path.file_name().unwrap());
            fs::copy(&path, &dest).map_err(|e| e.to_string())?;
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
