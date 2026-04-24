use anyhow::{Context, Result};
use models::data_sources::DataSource;
use reqwest::Client;
use sha2::{Digest, Sha256};
use std::time::Duration;

use crate::types::ScrappedItem;

#[derive(Clone)]
pub struct SourcePoller {
    http: Client,
}

impl SourcePoller {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http: Client::builder().timeout(Duration::from_secs(15)).build()?,
        })
    }

    pub async fn poll(&self, source: &DataSource) -> Result<Vec<ScrappedItem>> {
        match source.source_type.as_str() {
            "rss" => self.poll_rss(source).await,
            "web_page" => self.poll_web_page(source).await,
            "manual_note" | "placeholder_api" => Ok(Vec::new()),
            other => anyhow::bail!("unsupported source type {other}"),
        }
    }

    async fn fetch_body(&self, source: &DataSource) -> Result<String> {
        let url = source.url.as_deref().context("source url is required")?;
        self.http
            .get(url)
            .send()
            .await
            .with_context(|| format!("failed to fetch {url}"))?
            .error_for_status()
            .with_context(|| format!("source returned error status for {url}"))?
            .text()
            .await
            .context("failed to read response body")
    }

    async fn poll_web_page(&self, source: &DataSource) -> Result<Vec<ScrappedItem>> {
        let body = self.fetch_body(source).await?;
        let external_id = hash_text(&body);
        let title = extract_between(&body, "<title", "</title>")
            .and_then(|chunk| chunk.split('>').nth(1).map(str::to_string))
            .map(|value| html_unescape(value.trim()))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| source.url.clone().unwrap_or_else(|| source.name.clone()));
        Ok(vec![ScrappedItem {
            external_id,
            title,
            url: source.url.clone(),
            content: Some(body.clone()),
            summary: None,
            raw_payload: Some(json_string(&body)),
            published_at: None,
        }])
    }

    async fn poll_rss(&self, source: &DataSource) -> Result<Vec<ScrappedItem>> {
        let body = self.fetch_body(source).await?;
        let mut items = Vec::new();
        for chunk in body.split("<item").skip(1) {
            let item_body = match chunk.split("</item>").next() {
                Some(value) => value,
                None => continue,
            };
            let title = tag_text(item_body, "title").unwrap_or_else(|| source.name.clone());
            let link = tag_text(item_body, "link").or_else(|| source.url.clone());
            let guid = tag_text(item_body, "guid");
            let published_at = tag_text(item_body, "pubDate");
            let summary = tag_text(item_body, "description");
            let external_id = guid
                .or_else(|| link.clone())
                .unwrap_or_else(|| hash_text(&format!("{title}{published_at:?}")));
            items.push(ScrappedItem {
                external_id,
                title,
                url: link,
                content: summary.clone(),
                summary,
                raw_payload: Some(json_string(item_body)),
                published_at: None,
            });
        }

        if items.is_empty() {
            let external_id = hash_text(&body);
            items.push(ScrappedItem {
                external_id,
                title: format!("Raw feed snapshot: {}", source.name),
                url: source.url.clone(),
                content: Some(body.clone()),
                summary: Some("RSS parsing TODO: stored raw feed response".to_string()),
                raw_payload: Some(json_string(&body)),
                published_at: None,
            });
        }

        Ok(items)
    }
}

fn tag_text(body: &str, tag: &str) -> Option<String> {
    extract_between(body, &format!("<{tag}"), &format!("</{tag}>"))
        .and_then(|chunk| chunk.split('>').nth(1).map(str::to_string))
        .map(|value| html_unescape(value.trim()))
        .filter(|value| !value.is_empty())
}

fn extract_between(body: &str, start: &str, end: &str) -> Option<String> {
    let start_index = body.find(start)?;
    let rest = &body[start_index..];
    let end_index = rest.find(end)?;
    Some(rest[..end_index].to_string())
}

fn hash_text(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn html_unescape(value: &str) -> String {
    value
        .replace("<![CDATA[", "")
        .replace("]]>", "")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn json_string(value: &str) -> String {
    serde_json::json!({ "body": value }).to_string()
}
