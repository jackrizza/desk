#[derive(Clone, Debug)]
pub struct ScrappedItem {
    pub external_id: String,
    pub title: String,
    pub url: Option<String>,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub raw_payload: Option<String>,
    pub published_at: Option<String>,
}
