use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Frontmatter {
    title: String,
    excerpt: Option<String>,
    #[serde(default)]
    draft: bool,
}

fn rust_string(value: impl AsRef<str>) -> String {
    format!("{:?}", value.as_ref())
}

struct Post {
    slug: String,
    frontmatter: Frontmatter,
    content: String,
}

fn split_frontmatter<'a>(path: &Path, content: &'a str) -> (&'a str, &'a str) {
    let mut lines = content.split_inclusive('\n');
    let Some(opening_line) = lines.next() else {
        panic!("{} is empty", path.display());
    };

    if opening_line.trim_end_matches(['\r', '\n']) != "+++" {
        panic!(
            "{} must start with TOML frontmatter delimited by +++",
            path.display()
        );
    }

    let frontmatter_start = opening_line.len();
    let mut offset = frontmatter_start;

    for line in lines {
        let line_start = offset;
        offset += line.len();

        if line.trim_end_matches(['\r', '\n']) == "+++" {
            return (&content[frontmatter_start..line_start], &content[offset..]);
        }
    }

    panic!(
        "{} is missing closing +++ TOML frontmatter delimiter",
        path.display()
    );
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
        .map(|path| {
            println!("cargo:rerun-if-changed={}", path.display());

            let content = fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            let (frontmatter, content) = split_frontmatter(&path, &content);
            let frontmatter = toml::from_str::<Frontmatter>(frontmatter).unwrap_or_else(|err| {
                panic!("invalid TOML frontmatter in {}: {err}", path.display())
            });
            let slug = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_else(|| panic!("invalid blog post filename {}", path.display()))
                .to_owned();

            Post {
                slug,
                frontmatter,
                content: content.to_owned(),
            }
        })
        .collect::<Vec<_>>();

    posts.sort_by(|a, b| b.slug.cmp(&a.slug));

    let mut output = String::from("static BLOG_POSTS: &[BlogPost] = &[\n");
    for post in posts {
        output.push_str("    BlogPost {\n");
        output.push_str(&format!("        slug: {},\n", rust_string(post.slug)));
        output.push_str(&format!(
            "        title: {},\n",
            rust_string(post.frontmatter.title)
        ));
        match post.frontmatter.excerpt {
            Some(excerpt) => output.push_str(&format!(
                "        excerpt: Some({}),\n",
                rust_string(excerpt)
            )),
            None => output.push_str("        excerpt: None,\n"),
        }
        output.push_str(&format!("        draft: {},\n", post.frontmatter.draft));
        output.push_str(&format!(
            "        content: {},\n",
            rust_string(post.content)
        ));
        output.push_str("    },\n");
    }
    output.push_str("];\n");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_file = Path::new(&out_dir).join("blog_posts.rs");
    fs::write(out_file, output).unwrap();
}
