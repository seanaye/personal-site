use crate::domain::models::blog::BlogPost;

include!(concat!(env!("OUT_DIR"), "/blog_posts.rs"));

pub(crate) fn blog_post(slug: &str) -> Option<&'static BlogPost> {
    BLOG_POSTS.iter().find(|post| post.slug == slug)
}

pub(crate) fn can_show_post(post: &BlogPost, show_drafts: bool) -> bool {
    !post.draft || show_drafts
}
