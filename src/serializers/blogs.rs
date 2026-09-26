use crate::{
    entities_helper::BlogModel,
    serializers::pagination::Paginate,
    utils::serializer::deserialize_some,
};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct CreateBlogSerializer {
    pub title: String,
    pub slug: Option<String>,
    pub excerpt: Option<String>,
    pub content: String,
    pub tags: Option<String>,
    pub is_published: Option<bool>,
}

#[derive(Serialize, Debug, Clone)]
pub struct ReadBlogSerializer {
    blog_id: i64,
    title: String,
    slug: String,
    excerpt: Option<String>,
    content: String,
    tags: Option<String>,
    pub cover_image: Option<String>,
    is_published: bool,
    created_at: String,
    updated_at: String,
}

impl From<BlogModel> for ReadBlogSerializer {
    fn from(value: BlogModel) -> Self {
        ReadBlogSerializer {
            blog_id: value.blog_id,
            title: value.title,
            slug: value.slug,
            excerpt: value.excerpt,
            content: value.content,
            tags: value.tags,
            cover_image: value
                .cover_image
                .map(|_| format!("/blogs/{}/cover/image/", value.blog_id)),
            is_published: value.is_published,
            created_at: value.created_at.to_string(),
            updated_at: value.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct UpdateBlogSerializer {
    pub title: Option<String>,
    pub slug: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub excerpt: Option<Option<String>>,
    pub content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    pub tags: Option<Option<String>>,
    pub is_published: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlogFilter {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub is_published: Option<bool>,
    pub search: Option<String>,
    pub tag: Option<String>,
    pub sort_by: Option<String>,
}

impl Paginate for BlogFilter {
    fn limit(&self) -> u64 {
        self.page_size.unwrap_or(10)
    }

    fn page(&self) -> u64 {
        self.page.unwrap_or(1)
    }

    fn offset(&self) -> u64 {
        let page: u64 = self.page();
        if page == 0 {
            return 0;
        }
        (page - 1) * self.limit()
    }
}
