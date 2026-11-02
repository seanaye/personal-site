#[derive(Debug, Clone, Copy)]
pub(crate) struct BlogPost {
    pub slug: &'static str,
    pub title: &'static str,
    pub excerpt: Option<&'static str>,
    pub draft: bool,
    pub content: &'static str,
}
