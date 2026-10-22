//! Search queries: free text plus filter tokens. `@open` filters by status
//! (any of several), `#bug` by tag (all of several; `#bug` also matches
//! `#bug/ui`), and `in:work` by vault.

use crate::status::StatusFilter;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    pub text: String,
    pub statuses: Vec<StatusFilter>,
    pub tags: Vec<String>,
    /// `in:` tokens as typed (lower case); matched against vault ids and names.
    pub vaults: Vec<String>,
}

impl Query {
    pub fn parse(query: &str) -> Query {
        let mut q = Query::default();
        let mut rest = Vec::new();
        for word in query.split_whitespace() {
            if let Some(f) = word.strip_prefix('@').filter(|f| !f.is_empty()) {
                q.statuses.push(match f.to_lowercase().as_str() {
                    "active" => StatusFilter::Active,
                    "none" => StatusFilter::None,
                    other => StatusFilter::Named(other.to_string()),
                });
            } else if let Some(t) = word.strip_prefix('#').and_then(normalize_tag) {
                q.tags.push(t);
            } else if let Some(v) = word.strip_prefix("in:").filter(|v| !v.is_empty()) {
                q.vaults.push(v.to_lowercase());
            } else {
                rest.push(word);
            }
        }
        q.text = rest.join(" ");
        q
    }

    pub fn has_filters(&self) -> bool {
        !self.statuses.is_empty() || !self.tags.is_empty() || !self.vaults.is_empty()
    }

    /// Every `#tag` filter is on the note (a parent tag matches its children).
    pub fn tags_match(&self, tags: &[String]) -> bool {
        self.tags.iter().all(|want| tags.iter().any(|t| tag_within(t, want)))
    }
}

/// Is `tag` the same as `parent` or nested under it (`bug/ui` within `bug`)?
pub fn tag_within(tag: &str, parent: &str) -> bool {
    tag == parent || tag.strip_prefix(parent).is_some_and(|r| r.starts_with('/'))
}

/// A tag in canonical form: lower case, no leading `#`, made of letters,
/// digits, `-`, `_` and `/` (for nesting). Digits alone aren't a tag, so
/// `#123` stays an issue number.
pub fn normalize_tag(raw: &str) -> Option<String> {
    let t = raw.trim().trim_start_matches('#').trim_matches('/').to_lowercase();
    let ok = !t.is_empty()
        && t.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '/'))
        && !t.chars().all(|c| c.is_ascii_digit())
        && !t.contains("//");
    ok.then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tokens() {
        let q = Query::parse("#Bug @open in:Work rollout #123 date #");
        assert_eq!(q.text, "rollout #123 date #");
        assert_eq!(q.tags, vec!["bug"]);
        assert_eq!(q.statuses, vec![StatusFilter::Named("open".into())]);
        assert_eq!(q.vaults, vec!["work"]);
        assert!(q.has_filters());
        assert!(!Query::parse("plain words").has_filters());
    }

    #[test]
    fn tags() {
        assert_eq!(normalize_tag("#Research"), Some("research".into()));
        assert_eq!(normalize_tag("bug/UI/"), Some("bug/ui".into()));
        assert_eq!(normalize_tag("123"), None);
        assert_eq!(normalize_tag("a b"), None);
        assert_eq!(normalize_tag(""), None);
        assert!(tag_within("bug/ui", "bug"));
        assert!(!tag_within("bugs", "bug"));
        let q = Query::parse("#bug #ui");
        assert!(q.tags_match(&["bug/x".into(), "ui".into()]));
        assert!(!q.tags_match(&["bug".into()]));
    }
}
