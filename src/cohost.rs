use std::fs;
use anyhow::Context;
use camino::Utf8Path;
use serde::Deserialize;
use time::OffsetDateTime;
use url::Url;

#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct Project {
    pub handle: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub dek: String,
    pub description: String,
    #[serde(rename = "avatarURL")]
    pub avatar_url: Option<Url>,
    #[serde(rename = "avatarPreviewURL")]
    pub avatar_preview_url: Option<Url>,
    #[serde(rename = "headerURL")]
    pub header_url: Option<Url>,
    #[serde(rename = "headerPreviewURL")]
    pub header_preview_url: Option<Url>,
    #[serde(rename = "projectId")]
    pub project_id: u64,
    pub privacy: String,
    pub pronouns: String,
    pub url: String,
    pub flags: Vec<String>,
    #[serde(rename = "avatarShape")]
    pub avatar_shape: String,
    #[serde(rename = "loggedOutPostVisibility")]
    pub logged_out_post_visibility: String,
    #[serde(rename = "askSettings")]
    pub ask_settings: AskSettings,
    #[serde(rename = "frequentlyUsedTags")]
    pub frequently_used_tags: Vec<String>,
    #[serde(rename = "contactCard")]
    pub contact_card: Vec<String>,
    #[serde(rename = "deleteAfter")]
    pub delete_after: Option<String>,
    #[serde(rename = "isSelfProject")]
    pub is_self_project: bool,
}

impl Project {
    pub fn load(path: &Utf8Path) -> anyhow::Result<Self> {
        let file = fs::File::open(&path).with_context(|| format!("opening '{path}'"))?;
        let metadata: Self = serde_json::from_reader(&file).with_context(|| format!("parsing '{path} as Project JSON'"))?;

        Ok(metadata)
    }
}


#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct AskSettings {
    pub enabled: bool,
    #[serde(rename = "allowAnon")]
    pub allow_anon: bool,
    #[serde(rename = "requireLoggedInAnon")]
    pub require_logged_in_anon: bool,

}

#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct Post {
    #[serde(rename = "postId")]
    pub post_id: u64,
    pub headline: String,
    #[serde(rename = "publishedAt", with = "time::serde::rfc3339")]
    pub published_at: OffsetDateTime,
    pub state: u32,
    pub cws: Vec<String>,
    pub tags: Vec<String>,
    pub blocks: Vec<Block>,
    pub pinned: bool,
    #[serde(rename = "commentsLocked")]
    pub comments_locked: bool,
    #[serde(rename = "sharesLocked")]
    pub shares_locked: bool,
    #[serde(rename = "singlePostPageUrl")]
    pub single_post_page_url: Url,
}

#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum Block {
    #[serde(rename = "markdown")]
    Markdown { markdown: MarkdownBlock },
    #[serde(rename = "attachment")]
    Attachment { attachment: AttachmentBlock },
    #[serde(rename = "attachment-row")]
    AttachmentRow { attachments: Vec<Block> },
}

#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct MarkdownBlock {
    pub content: String,
}

#[allow(unused)]
#[derive(Clone, Debug, Deserialize)]
pub struct AttachmentBlock {
    pub kind: String,
    #[serde(rename = "fileURL")]
    pub file_url: Url,
    #[serde(rename = "previewURL")]
    pub preview_url: Url,
    #[serde(rename = "attachmentId")]
    pub attachment_id: String, // UUID
    #[serde(rename = "altText")]
    pub alt_text: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}