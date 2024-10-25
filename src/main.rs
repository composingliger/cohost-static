mod cohost;

use std::io::Write;
use std::fs;
use std::collections::HashSet;
use std::io::BufWriter;
use std::sync::Arc;

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use clap::Parser;
use percent_encoding::percent_decode_str;
use url::Url;

use crate::cohost::{Post, Project};

#[derive(Clone, Debug)]
struct Config {
    export_path: Utf8PathBuf,
    content_path: Utf8PathBuf,
    static_path: Utf8PathBuf,
}

#[derive(Clone, Debug)]
struct PostProcessor {
    path: Utf8PathBuf,
    config: Arc<Config>,
}

impl PostProcessor {
    pub fn new(path: impl Into<Utf8PathBuf>, config: Arc<Config>) -> Self {
        PostProcessor {
            path: path.into(),
            config,
        }
    }

    pub fn process(&self) -> anyhow::Result<()> {
        let post_json_path = self.path.join("post.json");
        let post_file = fs::File::open(&post_json_path).with_context(|| format!("opening '{post_json_path}'"))?;
        let post: Post = serde_json::from_reader(&post_file).expect("JSON data should be valid Post");

        let post_path = Utf8PathBuf::from(format!("{}.md", post.single_post_page_url.path()));
        let post_path = self.config.content_path.join(post_path.strip_prefix("/").expect("URL paths should always have '/'"));

        let post_parent_path = post_path.parent().expect("files should always have parent");
        fs::create_dir_all(&post_parent_path).with_context(|| format!("creating '{post_parent_path}'"))?;

        let output = fs::File::create(&post_path).with_context(|| format!("opening '{post_path}'"))?;
        let mut writer = BufWriter::new(output);

        eprintln!("\twriting '{post_path}'");
        self.write_post(&mut writer, &post)
    }

    fn write_post(&self, writer: &mut dyn Write, post: &Post) -> anyhow::Result<()> {
        let front_matter = {
            let title = post.headline.as_str();
            let date = post.published_at.format(&time::format_description::well_known::Rfc3339).expect("RFC 3339 should always be valid");
            let tags: Vec<_> = post.tags.iter().map(|t| toml::Value::from(t.as_str())).collect();
            let preview_image = find_preview_image(post).unwrap_or_default();

            toml::toml! {
                title = title
                date = date
                template = "project-post.html"

                [taxonomies]
                tags = tags

                [extra]
                preview_image = preview_image
            }
        };

        writeln!(writer, "+++\n{front_matter}\n+++\n")?;

        for block in &post.blocks {
            self.write_block(writer, block)?;
        }

        Ok(())
    }

    fn write_block(&self, mut writer: &mut dyn Write, block: &cohost::Block) -> anyhow::Result<()> {
        match block {
            cohost::Block::Markdown { markdown } => {
                if let Ok(url) = markdown.content.parse::<Url>() {
                    writeln!(writer, r#"<div class="embed">"#)?;
                    if url.domain().unwrap_or("#").ends_with("youtube.com") {
                        let v = url.query_pairs().into_iter().flat_map(|(k, v)| if k.as_ref() == "v" { Some( v.to_owned() ) } else { None } ).next().unwrap();
                        writeln!(writer, "{{{{ youtube(v=\"{}\") }}}}", escape_quotes(&v))?;
                    }

                    // actually a hyperlink
                    writeln!(writer, r#"<a class="url" target="_blank" rel="noopener noreferrer" href="{}">{}</a>"#, url, url)?;
                    writeln!(writer, r#"</div>"#)?;
                } else {
                    // regular markdown
                    writeln!(writer, "\n{}\n", markdown.content)?;
                }
            },
            cohost::Block::Attachment { attachment } => {
                writeln!(writer, r#"<div class="attachment">"#)?;

                let path = copy_static_resource(&self.config.static_path, &self.path, &attachment.file_url).expect("should be able to copy attachment resource");

                let alt_text = attachment.alt_text.as_deref().unwrap_or("");

                match attachment.kind.as_str() {
                    "image" => {
                        writeln!(writer, r#"{{{{ image(path="{}", alt="{}") }}}}"#, escape_quotes(path.as_str()), escape_quotes(alt_text))?;
                    },
                    "audio" => {
                        writeln!(writer, r#"{{{{ audio(path="{}") }}}}"#, escape_quotes(path.as_str()))?;
                    },
                    other => { eprint!("Unknown attachment type: {other}") },
                }
                writeln!(writer, r#"</div>"#)?;
            },
            cohost::Block::AttachmentRow { attachments } => {
                writeln!(writer, r#"<div class="row">"#)?;
                for attachment in attachments {
                    self.write_block(&mut writer, attachment)?;
                }
                writeln!(writer, r#"</div>"#)?;
            },
        }
        writeln!(writer, "")?;
        Ok(())
    }

}

/// Escape double-quotes to '&quot;'.
fn escape_quotes(s: &str) -> String {
    s.replace('"', "&quot;")
}

/// Decode percent-encoding string.
fn percent_decode(s: &str) -> String {
    percent_decode_str(s).decode_utf8().unwrap().to_string()
}

/// Try to find a preview image for this post.
fn find_preview_image(post: &Post) -> Option<&str> {
    post.blocks.iter().find_map(|b| {
        if let cohost::Block::Attachment { attachment } = b {
            if attachment.kind == "image" {
                return Some(attachment.file_url.path());
            }
        }

        None
    })
}

fn copy_static_resource(dest_prefix: &Utf8Path, source_dir: &Utf8Path, resource: &Url) -> anyhow::Result<Utf8PathBuf> {
    let filename_encoded = resource.path_segments().unwrap().last().expect("URL should always have at least one segment");
    let path = Utf8PathBuf::from(percent_decode(resource.path()));

    let src = source_dir.join(filename_encoded);
    let dest = dest_prefix.join(path.strip_prefix("/").expect("URL should always have '/'"));

    eprintln!("\tcopying '{}' to '{}'", src, dest);
    fs::create_dir_all(&dest.parent().expect("file should always have parent"))?;
    fs::copy(&src, &dest).with_context(|| format!("copying '{src}' to '{dest}'"))?;

    Ok(path)
}

/// Short URL representation with no scheme
fn short_url(url: Url) -> String {
    let host = url.host_str().unwrap_or_default();
    let path = url.path();

    format!("{host}{path}")
}

fn process_project(handle: &str, config: Arc<Config>) -> anyhow::Result<()> {
    let source_path = config.export_path.join("project").join(handle);
    eprintln!("Processing '{handle}' project (path: '{source_path}')");

    let metadata_path = source_path.join(format!("{handle}.json"));
    let project = Project::load(&metadata_path).with_context(|| format!("loading '{metadata_path}'"))?;

    let project_path = config.content_path.join(handle);

    let front_matter = {
        let handle = project.handle.as_str();
        let display_name = project.display_name.as_str();
        let dek = project.dek.as_str();
        let description = project.description.as_str();
        let header_url = project.header_url.as_ref().map(Url::path).unwrap_or_default();
        let avatar_url = project.avatar_url.as_ref().map(Url::path).unwrap_or_default();
        let pronouns = project.pronouns.as_str();
        let url = project.url.as_str();
        let url_short = if !url.is_empty() {
            short_url(Url::parse(&project.url)?)
        } else { Default::default() };
        let avatar_shape = project.avatar_shape.as_str();

        toml::toml! {
            sort_by = "date"
            template = "project-home.html"
            paginate_by = 20

            [extra]
            handle = handle
            display_name = display_name
            dek = dek
            description = description
            header_url = header_url
            avatar_url = avatar_url
            pronouns = pronouns
            url = url
            url_short = url_short
            avatar_shape = avatar_shape
        }
    };

    let front_matter_path = project_path.join("_index.md");
    eprintln!("\twriting '{front_matter_path}'");
    fs::create_dir_all(&project_path).expect("should be able to create project directory");
    fs::write(front_matter_path, format!("+++\n{front_matter}\n+++\n")).expect("");

    if let Some(avatar_url) = &project.avatar_url {
        copy_static_resource(&config.static_path, &source_path, avatar_url)?;
    }

    if let Some(header_url) = &project.header_url {
        copy_static_resource(&config.static_path, &source_path, header_url)?;
    }

    let public_posts_path = source_path.join("posts").join("published");
    for dirent in fs::read_dir(&public_posts_path).with_context(|| format!("reading '{public_posts_path}'"))? {
        let dirent = dirent?;
        if !dirent.file_type()?.is_dir() {
            continue;
        }

        let path = Utf8PathBuf::from_path_buf(dirent.path()).expect("path should be valid UTF-8");

        PostProcessor::new(path, Arc::clone(&config)).process()?;
    }

    let post_index = toml::toml! {
        sort_by = "date"
        transparent = true
        template = "404.html"
        page_template = "blog-page.html"
    };

    let post_index_path = config.content_path.join(handle).join("post").join("_index.md");
    eprintln!("\twriting '{post_index_path}'");
    fs::write(post_index_path, format!("+++\n{post_index}\n+++"))?;

    Ok(())
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = Utf8PathBuf::from("zola"))]
    output_path: Utf8PathBuf,

    #[arg(short, long)]
    projects: Option<String>,

    export_path: Utf8PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config = Arc::new(
        Config {
            export_path: args.export_path,
            content_path: args.output_path.join("content"),
            static_path: args.output_path.join("static"),
        }
    );

    if !fs::exists(config.export_path.join("user.json"))? {
        anyhow::bail!("Could not find 'user.json' in {} - is this a valid cohost export?", config.export_path);
    }

    let project_path = config.export_path.join("project");
    let all_projects: Vec<_> = fs::read_dir(&project_path)?.filter_map(|res| {
        if let Ok(dirent) = res {
            if dirent.file_type().unwrap().is_dir() {
                return Some(dirent.file_name().into_string().unwrap());
            }
        }

        None
    }).collect();

    let projects: Vec<_> = if let Some(projects) = &args.projects {
        let projects: HashSet<_> = projects.split(',').collect();

        all_projects.into_iter().filter(|p| projects.contains(p.as_str())).collect()
    } else {
        all_projects
    };

    for project_handle in &projects {
        process_project(&project_handle, Arc::clone(&config)).with_context(|| format!("processing project '{}'", project_handle))?;
    }

    Ok(())
}
