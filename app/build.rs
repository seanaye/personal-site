use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn title_from_slug(slug: &str) -> String {
    slug.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn markdown_title(markdown: &str) -> Option<String> {
    markdown.lines().find_map(|line| {
        line.strip_prefix("# ")
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn markdown_excerpt(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .map(ToOwned::to_owned)
}

fn rust_string(value: impl AsRef<str>) -> String {
    format!("{:?}", value.as_ref())
}

struct Post {
    slug: String,
    title: String,
    excerpt: Option<String>,
    path: PathBuf,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let blog_dir = manifest_dir.join("../blog");
    println!("cargo:rerun-if-changed={}", blog_dir.display());

    let mut posts = fs::read_dir(&blog_dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .filter_map(|path| {
            println!("cargo:rerun-if-changed={}", path.display());

            let content = fs::read_to_string(&path).ok()?;
            let slug = path.file_stem()?.to_str()?.to_owned();
            let title = markdown_title(&content).unwrap_or_else(|| title_from_slug(&slug));
            let excerpt = markdown_excerpt(&content);

            Some(Post {
                slug,
                title,
                excerpt,
                path,
            })
        })
        .collect::<Vec<_>>();

    posts.sort_by(|a, b| b.slug.cmp(&a.slug));

    let mut output = String::from("static BLOG_POSTS: &[BlogPost] = &[\n");
    for post in posts {
        let path = post.path.canonicalize().unwrap_or(post.path);
        output.push_str("    BlogPost {\n");
        output.push_str(&format!("        slug: {},\n", rust_string(post.slug)));
        output.push_str(&format!("        title: {},\n", rust_string(post.title)));
        match post.excerpt {
            Some(excerpt) => output.push_str(&format!(
                "        excerpt: Some({}),\n",
                rust_string(excerpt)
            )),
            None => output.push_str("        excerpt: None,\n"),
        }
        output.push_str(&format!(
            "        content: include_str!({}),\n",
            rust_string(path.to_string_lossy())
        ));
        output.push_str("    },\n");
    }
    output.push_str("];\n");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file = Path::new(&out_dir).join("blog_posts.rs");
    fs::write(out_file, output).unwrap();
}
