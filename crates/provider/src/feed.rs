// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Feed provider — probes, fetches, and parses RSS, Atom, and JSON Feed.
//!
//! Providers produce observations, not editorial decisions. This module keeps
//! feed transport and parsing generic: downstream packs decide source trust,
//! rights, gates, and domain meaning.

use std::sync::Arc;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::fetch::HttpFetchProvider;
use crate::search::{WebFetchBackend, WebFetchError, WebFetchRequest, WebFetchResponse};

/// Feed format identified by probe hints or parser detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedFormat {
    Rss,
    Atom,
    JsonFeed,
    Unknown,
}

/// How a candidate feed endpoint was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedDiscoverySource {
    AlternateLink,
    CommonPath,
    DirectUrl,
}

/// Request to discover likely feed endpoints for a site or direct feed URL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedProbeRequest {
    pub url: String,
    #[serde(default = "default_probe_common_paths")]
    pub probe_common_paths: bool,
    #[serde(default = "default_max_candidates")]
    pub max_candidates: usize,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

fn default_probe_common_paths() -> bool {
    true
}

fn default_max_candidates() -> usize {
    16
}

fn default_max_bytes() -> usize {
    1_048_576
}

fn default_timeout_ms() -> u64 {
    30_000
}

impl FeedProbeRequest {
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            probe_common_paths: default_probe_common_paths(),
            max_candidates: default_max_candidates(),
            max_bytes: default_max_bytes(),
            timeout_ms: default_timeout_ms(),
        }
    }
}

/// Candidate feed endpoint discovered during probing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedEndpointCandidate {
    pub url: String,
    pub format_hint: FeedFormat,
    pub discovery_source: FeedDiscoverySource,
    pub confidence_bps: u16,
}

/// Feed probe response. Candidates are observations and require downstream
/// promotion before use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedProbeResponse {
    pub provider: String,
    pub input_url: String,
    pub candidates: Vec<FeedEndpointCandidate>,
}

/// Request to fetch and parse one feed endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedFetchRequest {
    pub url: String,
    #[serde(default)]
    pub headers: Vec<(String, String)>,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

impl FeedFetchRequest {
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            headers: Vec::new(),
            max_bytes: default_max_bytes(),
            timeout_ms: default_timeout_ms(),
        }
    }

    #[must_use]
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

impl From<&FeedFetchRequest> for WebFetchRequest {
    fn from(request: &FeedFetchRequest) -> Self {
        let mut fetch = WebFetchRequest::new(&request.url)
            .with_max_bytes(request.max_bytes)
            .with_timeout_ms(request.timeout_ms);
        for (name, value) in &request.headers {
            fetch = fetch.with_header(name, value);
        }
        fetch
    }
}

/// Normalized feed item observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedItem {
    pub id: Option<String>,
    pub title: Option<String>,
    pub link: Option<String>,
    pub summary: Option<String>,
    pub published_at: Option<String>,
    pub updated_at: Option<String>,
    pub authors: Vec<String>,
    pub categories: Vec<String>,
    pub item_hash: String,
}

/// Feed fetch response. Raw body is retained so callers can store the exact
/// representation used to derive normalized items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedFetchResponse {
    pub provider: String,
    pub url: String,
    pub status: u16,
    pub content_type: Option<String>,
    pub format: FeedFormat,
    pub raw_hash: String,
    pub raw_body: String,
    pub truncated: bool,
    pub feed_title: Option<String>,
    pub feed_link: Option<String>,
    pub feed_updated_at: Option<String>,
    pub items: Vec<FeedItem>,
}

/// Feed provider errors.
#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("fetch error: {0}")]
    Fetch(String),
    #[error("invalid url: {0}")]
    InvalidUrl(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("unsupported feed format")]
    UnsupportedFormat,
}

impl From<WebFetchError> for FeedError {
    fn from(error: WebFetchError) -> Self {
        Self::Fetch(error.to_string())
    }
}

/// Executable contract for provider-local feed adapters.
pub trait FeedFetchBackend: Send + Sync {
    fn provider_name(&self) -> &'static str;

    fn probe(&self, request: &FeedProbeRequest) -> Result<FeedProbeResponse, FeedError>;

    fn fetch_feed(&self, request: &FeedFetchRequest) -> Result<FeedFetchResponse, FeedError>;
}

/// HTTP-backed feed provider.
#[derive(Clone)]
pub struct HttpFeedProvider {
    fetch_backend: Arc<dyn WebFetchBackend>,
}

impl std::fmt::Debug for HttpFeedProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpFeedProvider").finish_non_exhaustive()
    }
}

impl Default for HttpFeedProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpFeedProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {
            fetch_backend: Arc::new(HttpFetchProvider::new()),
        }
    }

    #[must_use]
    pub fn with_fetch_backend(fetch_backend: Arc<dyn WebFetchBackend>) -> Self {
        Self { fetch_backend }
    }
}

impl FeedFetchBackend for HttpFeedProvider {
    fn provider_name(&self) -> &'static str {
        "http-feed"
    }

    fn probe(&self, request: &FeedProbeRequest) -> Result<FeedProbeResponse, FeedError> {
        let input_url =
            Url::parse(&request.url).map_err(|error| FeedError::InvalidUrl(error.to_string()))?;
        let mut candidates = Vec::new();

        if looks_like_feed_url(input_url.path()) {
            candidates.push(FeedEndpointCandidate {
                url: input_url.to_string(),
                format_hint: format_hint_from_url(input_url.path()),
                discovery_source: FeedDiscoverySource::DirectUrl,
                confidence_bps: 9_000,
            });
        }

        let fetch = WebFetchRequest::new(input_url.as_str())
            .with_max_bytes(request.max_bytes)
            .with_timeout_ms(request.timeout_ms);
        if let Ok(response) = self.fetch_backend.fetch(&fetch) {
            candidates.extend(discover_alternate_links(&response.body, &response.url)?);
        }

        if request.probe_common_paths {
            candidates.extend(common_feed_candidates(&input_url));
        }

        dedup_candidates(&mut candidates);
        candidates.truncate(request.max_candidates);

        Ok(FeedProbeResponse {
            provider: self.provider_name().into(),
            input_url: request.url.clone(),
            candidates,
        })
    }

    fn fetch_feed(&self, request: &FeedFetchRequest) -> Result<FeedFetchResponse, FeedError> {
        let fetch_request = WebFetchRequest::from(request);
        let response = self.fetch_backend.fetch(&fetch_request)?;
        parse_feed_response(self.provider_name(), response)
    }
}

fn parse_feed_response(
    provider_name: &str,
    response: WebFetchResponse,
) -> Result<FeedFetchResponse, FeedError> {
    let raw_hash = sha256(&response.body);
    let parsed = parse_feed(&response.body)?;

    Ok(FeedFetchResponse {
        provider: provider_name.into(),
        url: response.url,
        status: response.status,
        content_type: response.content_type,
        format: parsed.format,
        raw_hash,
        raw_body: response.body,
        truncated: response.truncated,
        feed_title: parsed.feed_title,
        feed_link: parsed.feed_link,
        feed_updated_at: parsed.feed_updated_at,
        items: parsed.items,
    })
}

#[derive(Debug, Default)]
struct ParsedFeed {
    format: FeedFormat,
    feed_title: Option<String>,
    feed_link: Option<String>,
    feed_updated_at: Option<String>,
    items: Vec<FeedItem>,
}

impl Default for FeedFormat {
    fn default() -> Self {
        Self::Unknown
    }
}

fn parse_feed(body: &str) -> Result<ParsedFeed, FeedError> {
    if let Ok(feed) = parse_json_feed(body) {
        return Ok(feed);
    }

    parse_xml_feed(body)
}

#[derive(Debug, Deserialize)]
struct JsonFeed {
    title: Option<String>,
    home_page_url: Option<String>,
    feed_url: Option<String>,
    items: Vec<JsonFeedItem>,
}

#[derive(Debug, Deserialize)]
struct JsonFeedItem {
    id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    external_url: Option<String>,
    summary: Option<String>,
    content_text: Option<String>,
    date_published: Option<String>,
    date_modified: Option<String>,
    author: Option<JsonFeedAuthor>,
    authors: Option<Vec<JsonFeedAuthor>>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct JsonFeedAuthor {
    name: Option<String>,
}

fn parse_json_feed(body: &str) -> Result<ParsedFeed, FeedError> {
    let json: JsonFeed =
        serde_json::from_str(body).map_err(|error| FeedError::Parse(error.to_string()))?;
    let feed_link = json.home_page_url.or(json.feed_url);
    let items = json
        .items
        .into_iter()
        .map(|item| {
            let mut authors = item
                .authors
                .unwrap_or_default()
                .into_iter()
                .filter_map(|author| author.name)
                .collect::<Vec<_>>();
            if let Some(author) = item.author.and_then(|author| author.name) {
                authors.push(author);
            }
            authors.sort();
            authors.dedup();
            let summary = item.summary.or(item.content_text);
            let link = item.url.or(item.external_url);
            let item_hash = item_hash(&item.id, &item.title, &link, &summary);

            FeedItem {
                id: item.id,
                title: item.title,
                link,
                summary,
                published_at: item.date_published,
                updated_at: item.date_modified,
                authors,
                categories: item.tags.unwrap_or_default(),
                item_hash,
            }
        })
        .collect();

    Ok(ParsedFeed {
        format: FeedFormat::JsonFeed,
        feed_title: json.title,
        feed_link,
        feed_updated_at: None,
        items,
    })
}

#[derive(Debug, Default)]
struct FeedItemBuilder {
    id: Option<String>,
    title: Option<String>,
    link: Option<String>,
    summary: Option<String>,
    published_at: Option<String>,
    updated_at: Option<String>,
    authors: Vec<String>,
    categories: Vec<String>,
}

impl FeedItemBuilder {
    fn build(self) -> FeedItem {
        let item_hash = item_hash(&self.id, &self.title, &self.link, &self.summary);
        FeedItem {
            id: self.id,
            title: self.title,
            link: self.link,
            summary: self.summary,
            published_at: self.published_at,
            updated_at: self.updated_at,
            authors: self.authors,
            categories: self.categories,
            item_hash,
        }
    }
}

fn parse_xml_feed(body: &str) -> Result<ParsedFeed, FeedError> {
    let mut reader = Reader::from_str(body);
    reader.config_mut().trim_text(true);
    let mut feed = ParsedFeed::default();
    let mut current_item: Option<FeedItemBuilder> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(start)) => {
                let name = local_name(start.name().as_ref());
                match name.as_str() {
                    "rss" | "rdf" => feed.format = FeedFormat::Rss,
                    "feed" => feed.format = FeedFormat::Atom,
                    "item" | "entry" => current_item = Some(FeedItemBuilder::default()),
                    _ if current_item.is_some() => {
                        read_item_start(&mut reader, &start, &name, current_item.as_mut());
                    }
                    "title" => feed.feed_title = read_text(&mut reader, &start)?,
                    "link" => {
                        if feed.format == FeedFormat::Atom {
                            feed.feed_link = attr_value(&reader, &start, b"href");
                        } else {
                            feed.feed_link = read_text(&mut reader, &start)?;
                        }
                    }
                    "updated" | "lastbuilddate" => {
                        feed.feed_updated_at = read_text(&mut reader, &start)?
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(empty)) => {
                let name = local_name(empty.name().as_ref());
                if name == "link" {
                    if let Some(item) = current_item.as_mut() {
                        item.link = item
                            .link
                            .take()
                            .or_else(|| attr_value(&reader, &empty, b"href"));
                    } else if feed.format == FeedFormat::Atom {
                        feed.feed_link = feed
                            .feed_link
                            .take()
                            .or_else(|| attr_value(&reader, &empty, b"href"));
                    }
                }
            }
            Ok(Event::End(end)) => {
                let name = local_name(end.name().as_ref());
                if matches!(name.as_str(), "item" | "entry") {
                    if let Some(item) = current_item.take() {
                        feed.items.push(item.build());
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(error) => return Err(FeedError::Parse(error.to_string())),
            _ => {}
        }
    }

    if feed.format == FeedFormat::Unknown {
        return Err(FeedError::UnsupportedFormat);
    }

    Ok(feed)
}

fn read_item_start(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart<'_>,
    name: &str,
    item: Option<&mut FeedItemBuilder>,
) {
    let Some(item) = item else {
        return;
    };

    match name {
        "title" => item.title = read_text(reader, start).ok().flatten(),
        "link" => {
            item.link = attr_value(reader, start, b"href")
                .or_else(|| read_text(reader, start).ok().flatten());
        }
        "guid" | "id" => item.id = read_text(reader, start).ok().flatten(),
        "description" | "summary" | "content" | "encoded" => {
            if item.summary.is_none() {
                item.summary = read_text(reader, start).ok().flatten();
            }
        }
        "pubdate" | "published" => item.published_at = read_text(reader, start).ok().flatten(),
        "updated" => item.updated_at = read_text(reader, start).ok().flatten(),
        "creator" | "author" | "name" => {
            if let Some(author) = read_text(reader, start).ok().flatten() {
                item.authors.push(author);
            }
        }
        "category" => {
            if let Some(category) = read_text(reader, start).ok().flatten() {
                item.categories.push(category);
            }
        }
        _ => {}
    }
}

fn read_text(
    reader: &mut Reader<&[u8]>,
    start: &BytesStart<'_>,
) -> Result<Option<String>, FeedError> {
    reader
        .read_text(start.name())
        .map(|text| {
            let trimmed = text.trim();
            (!trimmed.is_empty()).then_some(trimmed.to_string())
        })
        .map_err(|error| FeedError::Parse(error.to_string()))
}

fn local_name(name: &[u8]) -> String {
    let raw = String::from_utf8_lossy(name).to_ascii_lowercase();
    raw.rsplit(':').next().unwrap_or(&raw).to_string()
}

fn attr_value(reader: &Reader<&[u8]>, start: &BytesStart<'_>, key: &[u8]) -> Option<String> {
    start
        .attributes()
        .flatten()
        .find(|attribute| local_name(attribute.key.as_ref()).as_bytes() == key)
        .and_then(|attribute| {
            attribute
                .decode_and_unescape_value(reader.decoder())
                .ok()
                .map(|value| value.into_owned())
        })
}

fn discover_alternate_links(
    body: &str,
    base_url: &str,
) -> Result<Vec<FeedEndpointCandidate>, FeedError> {
    let base = Url::parse(base_url).map_err(|error| FeedError::InvalidUrl(error.to_string()))?;
    let link_re = regex_lite::Regex::new(r#"(?is)<link\s+[^>]*>"#)
        .map_err(|error| FeedError::Parse(error.to_string()))?;
    let attr_re = regex_lite::Regex::new(r#"(?is)([a-zA-Z_:.-]+)\s*=\s*["']([^"']+)["']"#)
        .map_err(|error| FeedError::Parse(error.to_string()))?;
    let mut candidates = Vec::new();

    for link_match in link_re.find_iter(body) {
        let tag = link_match.as_str();
        let mut rel = None;
        let mut content_type = None;
        let mut href = None;

        for capture in attr_re.captures_iter(tag) {
            let Some(name) = capture.get(1) else {
                continue;
            };
            let Some(value) = capture.get(2) else {
                continue;
            };
            match name.as_str().to_ascii_lowercase().as_str() {
                "rel" => rel = Some(value.as_str().to_ascii_lowercase()),
                "type" => content_type = Some(value.as_str().to_ascii_lowercase()),
                "href" => href = Some(value.as_str().to_string()),
                _ => {}
            }
        }

        if !rel.as_deref().unwrap_or_default().contains("alternate") {
            continue;
        }
        let format_hint = content_type
            .as_deref()
            .map(format_hint_from_content_type)
            .unwrap_or(FeedFormat::Unknown);
        if format_hint == FeedFormat::Unknown {
            continue;
        }
        let Some(href) = href else {
            continue;
        };
        let url = base
            .join(&href)
            .map_err(|error| FeedError::InvalidUrl(error.to_string()))?;
        candidates.push(FeedEndpointCandidate {
            url: url.to_string(),
            format_hint,
            discovery_source: FeedDiscoverySource::AlternateLink,
            confidence_bps: 8_500,
        });
    }

    Ok(candidates)
}

fn common_feed_candidates(base_url: &Url) -> Vec<FeedEndpointCandidate> {
    let common_paths = [
        ("/feed", FeedFormat::Rss),
        ("/feed/", FeedFormat::Rss),
        ("/rss", FeedFormat::Rss),
        ("/rss.xml", FeedFormat::Rss),
        ("/feed.xml", FeedFormat::Rss),
        ("/atom.xml", FeedFormat::Atom),
        ("/index.xml", FeedFormat::Rss),
        ("/feed.json", FeedFormat::JsonFeed),
    ];
    common_paths
        .into_iter()
        .filter_map(|(path, format_hint)| {
            base_url.join(path).ok().map(|url| FeedEndpointCandidate {
                url: url.to_string(),
                format_hint,
                discovery_source: FeedDiscoverySource::CommonPath,
                confidence_bps: 4_000,
            })
        })
        .collect()
}

fn dedup_candidates(candidates: &mut Vec<FeedEndpointCandidate>) {
    candidates.sort_by(|a, b| {
        b.confidence_bps
            .cmp(&a.confidence_bps)
            .then_with(|| a.url.cmp(&b.url))
    });
    candidates.dedup_by(|a, b| a.url == b.url);
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn looks_like_feed_url(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.ends_with(".rss")
        || path.ends_with(".xml")
        || path.ends_with(".atom")
        || path.ends_with(".json")
        || path.ends_with("/feed")
        || path.ends_with("/feed/")
        || path.ends_with("/rss")
}

#[allow(clippy::case_sensitive_file_extension_comparisons)]
fn format_hint_from_url(path: &str) -> FeedFormat {
    let path = path.to_ascii_lowercase();
    if path.ends_with(".json") {
        FeedFormat::JsonFeed
    } else if path.contains("atom") {
        FeedFormat::Atom
    } else if looks_like_feed_url(&path) {
        FeedFormat::Rss
    } else {
        FeedFormat::Unknown
    }
}

fn format_hint_from_content_type(content_type: &str) -> FeedFormat {
    if content_type.contains("json") {
        FeedFormat::JsonFeed
    } else if content_type.contains("atom") {
        FeedFormat::Atom
    } else if content_type.contains("rss") || content_type.contains("xml") {
        FeedFormat::Rss
    } else {
        FeedFormat::Unknown
    }
}

fn item_hash(
    id: &Option<String>,
    title: &Option<String>,
    link: &Option<String>,
    summary: &Option<String>,
) -> String {
    sha256(&format!("{id:?}\n{title:?}\n{link:?}\n{summary:?}"))
}

fn sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct StaticFetchBackend {
        response: WebFetchResponse,
    }

    impl WebFetchBackend for StaticFetchBackend {
        fn provider_name(&self) -> &'static str {
            "static"
        }

        fn fetch(&self, _request: &WebFetchRequest) -> Result<WebFetchResponse, WebFetchError> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn parses_rss_items_and_hashes_raw_body() {
        let response = WebFetchResponse {
            url: "https://example.test/feed.xml".into(),
            status: 200,
            content_type: Some("application/rss+xml".into()),
            body: r#"
                <rss version="2.0">
                  <channel>
                    <title>Local News</title>
                    <link>https://example.test</link>
                    <item>
                      <guid>abc</guid>
                      <title>Council update</title>
                      <link>https://example.test/a</link>
                      <description>Short summary.</description>
                      <pubDate>Thu, 30 Apr 2026 08:00:00 GMT</pubDate>
                      <category>Civic</category>
                    </item>
                  </channel>
                </rss>
            "#
            .into(),
            truncated: false,
        };
        let provider =
            HttpFeedProvider::with_fetch_backend(Arc::new(StaticFetchBackend { response }));
        let parsed = provider
            .fetch_feed(&FeedFetchRequest::new("https://example.test/feed.xml"))
            .unwrap();

        assert_eq!(parsed.format, FeedFormat::Rss);
        assert_eq!(parsed.feed_title.as_deref(), Some("Local News"));
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].title.as_deref(), Some("Council update"));
        assert_eq!(parsed.items[0].id.as_deref(), Some("abc"));
        assert!(parsed.raw_hash.starts_with("sha256:"));
        assert!(parsed.items[0].item_hash.starts_with("sha256:"));
    }

    #[test]
    fn parses_atom_entries() {
        let response = WebFetchResponse {
            url: "https://example.test/atom.xml".into(),
            status: 200,
            content_type: Some("application/atom+xml".into()),
            body: r#"
                <feed xmlns="http://www.w3.org/2005/Atom">
                  <title>Atom News</title>
                  <link href="https://example.test"/>
                  <entry>
                    <id>tag:example.test,2026:1</id>
                    <title>Match report</title>
                    <link href="https://example.test/match"/>
                    <summary>Färjestad update.</summary>
                    <updated>2026-04-30T10:00:00Z</updated>
                  </entry>
                </feed>
            "#
            .into(),
            truncated: false,
        };
        let provider =
            HttpFeedProvider::with_fetch_backend(Arc::new(StaticFetchBackend { response }));
        let parsed = provider
            .fetch_feed(&FeedFetchRequest::new("https://example.test/atom.xml"))
            .unwrap();

        assert_eq!(parsed.format, FeedFormat::Atom);
        assert_eq!(
            parsed.items[0].link.as_deref(),
            Some("https://example.test/match")
        );
        assert_eq!(
            parsed.items[0].updated_at.as_deref(),
            Some("2026-04-30T10:00:00Z")
        );
    }

    #[test]
    fn parses_json_feed_items() {
        let response = WebFetchResponse {
            url: "https://example.test/feed.json".into(),
            status: 200,
            content_type: Some("application/feed+json".into()),
            body: r#"
                {
                  "version": "https://jsonfeed.org/version/1.1",
                  "title": "JSON News",
                  "home_page_url": "https://example.test",
                  "items": [
                    {
                      "id": "1",
                      "url": "https://example.test/1",
                      "title": "Coffee company expands",
                      "summary": "Local business update.",
                      "date_published": "2026-04-30T09:00:00Z",
                      "tags": ["business"]
                    }
                  ]
                }
            "#
            .into(),
            truncated: false,
        };
        let provider =
            HttpFeedProvider::with_fetch_backend(Arc::new(StaticFetchBackend { response }));
        let parsed = provider
            .fetch_feed(&FeedFetchRequest::new("https://example.test/feed.json"))
            .unwrap();

        assert_eq!(parsed.format, FeedFormat::JsonFeed);
        assert_eq!(parsed.feed_title.as_deref(), Some("JSON News"));
        assert_eq!(parsed.items[0].categories, vec!["business"]);
    }

    #[test]
    fn probe_discovers_alternate_feed_links_and_common_paths() {
        let response = WebFetchResponse {
            url: "https://example.test/".into(),
            status: 200,
            content_type: Some("text/html".into()),
            body: r#"
                <html>
                  <head>
                    <link rel="alternate" type="application/rss+xml" href="/rss.xml">
                    <link rel="alternate" type="application/atom+xml" href="https://example.test/atom.xml">
                  </head>
                </html>
            "#
            .into(),
            truncated: false,
        };
        let provider =
            HttpFeedProvider::with_fetch_backend(Arc::new(StaticFetchBackend { response }));
        let probe = provider
            .probe(&FeedProbeRequest::new("https://example.test/"))
            .unwrap();

        assert!(probe.candidates.iter().any(|candidate| {
            candidate.discovery_source == FeedDiscoverySource::AlternateLink
                && candidate.url == "https://example.test/rss.xml"
        }));
        assert!(probe.candidates.iter().any(|candidate| {
            candidate.discovery_source == FeedDiscoverySource::CommonPath
                && candidate.url == "https://example.test/feed"
        }));
    }
}
