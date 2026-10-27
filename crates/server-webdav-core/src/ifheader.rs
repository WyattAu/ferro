//! RFC 4918 §10.4 `If` header parsing and evaluation.
//!
//! Companion to YP-WEBDAV-PROTOCOL-001 Rev 1.1 (R3-5/R4-L4). Replaces the
//! naive first-`<`…first-`>` extractor: supports untagged and tagged
//! lists, `Not`, multiple parenthesized lists (OR of ANDs), and tolerant
//! whitespace. Lock-token submission collects non-negated state tokens.

/// One condition inside a list: optional `Not` + either a state token
/// (`<opaquelocktoken:…>`) or an entity-tag condition (`["…"]`, RFC 4918
/// §10.4.4). For ETag conditions `token` carries the tag text as written
/// (usually including the entity-tag quotes) and `etag` is true.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    pub negate: bool,
    pub token: String,
    pub etag: bool,
}

/// A parenthesized list of conditions (AND semantics).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionList {
    pub conditions: Vec<Condition>,
}

/// A tagged group: resource tag + lists (OR of ANDs, scoped to resource).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaggedGroup {
    pub resource: String,
    pub lists: Vec<ConditionList>,
}

/// Parsed If header: either untagged lists or tagged groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IfHeader {
    Untagged(Vec<ConditionList>),
    Tagged(Vec<TaggedGroup>),
}

impl IfHeader {
    /// All non-negated state tokens — the tokens the client claims to hold.
    #[must_use]
    pub fn submitted_tokens(&self) -> Vec<String> {
        let mut out = Vec::new();
        fn push(lists: &[ConditionList], out: &mut Vec<String>) {
            for list in lists {
                for c in &list.conditions {
                    // ETag conditions are not state tokens; lock-token
                    // submission collects lock tokens only.
                    if !c.etag && !c.negate && !out.contains(&c.token) {
                        out.push(c.token.clone());
                    }
                }
            }
        }
        match self {
            IfHeader::Untagged(lists) => push(lists, &mut out),
            IfHeader::Tagged(groups) => {
                for g in groups {
                    push(&g.lists, &mut out);
                }
            }
        }
        out
    }

    /// Evaluate against a request resource and the set of current state
    /// tokens (lock tokens the server knows are valid for this principal).
    ///
    /// Untagged: at least one list must be entirely satisfied.
    /// Tagged: groups whose resource matches are evaluated the same way;
    /// groups for other resources are ignored. If no group matches, the
    /// header passes (RFC 4918 §10.4.3 — tagged lists only apply to their
    /// listed resources).
    #[must_use]
    pub fn evaluate(&self, resource: &str, current_tokens: &[String], current_etag: Option<&str>) -> bool {
        fn list_holds(list: &ConditionList, current: &[String], etag: Option<&str>) -> bool {
            list.conditions.iter().all(|c| {
                let present = if c.etag {
                    etag.is_some_and(|cur| norm_etag(cur) == norm_etag(&c.token))
                } else {
                    current.iter().any(|t| t == &c.token)
                };
                c.negate ^ present
            })
        }
        match self {
            IfHeader::Untagged(lists) => lists.iter().any(|l| list_holds(l, current_tokens, current_etag)),
            IfHeader::Tagged(groups) => {
                let matching: Vec<&TaggedGroup> = groups
                    .iter()
                    .filter(|g| tag_path(&g.resource).trim_end_matches('/') == resource.trim_end_matches('/'))
                    .collect();
                if matching.is_empty() {
                    return true;
                }
                matching
                    .iter()
                    .any(|g| g.lists.iter().any(|l| list_holds(l, current_tokens, current_etag)))
            }
        }
    }
}

/// Path component of a resource tag: `<scheme://host>/path` → `/path`;
/// a bare path passes through (trimmed of any trailing slash comparison is
/// done by the caller).
/// Normalize an entity-tag for §10.4.4 comparison: strip one layer of
/// surrounding double quotes so `["abc"]` in the header matches a stored
/// etag of `"abc"` or `abc` (clients transmit the quoted form; Ferro's
/// metadata etags are bare).
fn norm_etag(t: &str) -> &str {
    // Strip every layer of surrounding quotes: Ferro's stored etags have
    // historically embedded their own quotes, so the header value
    // (`["abc"]`) can arrive double-quoted (`""abc""`) after transport.
    let mut t = t.trim();
    while let Some(inner) = t.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
        t = inner;
    }
    t
}

fn tag_path(tag: &str) -> &str {
    if let Some(idx) = tag.find("://") {
        let rest = &tag[idx + 3..];
        match rest.find('/') {
            Some(slash) => &rest[slash..],
            None => "/",
        }
    } else {
        tag
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Self { s: s.as_bytes(), i: 0 }
    }

    fn skip_ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, b: u8) -> bool {
        self.skip_ws();
        if self.i < self.s.len() && self.s[self.i] == b {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn peek_word(&mut self, word: &[u8]) -> bool {
        self.skip_ws();
        self.s[self.i..].starts_with(word)
    }

    /// Read until `end` byte (exclusive), returning the trimmed slice.
    fn until(&mut self, end: u8) -> Option<&'a str> {
        self.skip_ws();
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i] != end {
            self.i += 1;
        }
        if self.i >= self.s.len() {
            return None;
        }
        let out = std::str::from_utf8(&self.s[start..self.i]).ok()?;
        self.i += 1; // consume end
        Some(out.trim())
    }

    fn condition(&mut self) -> Option<Condition> {
        let negate = self.peek_word(b"Not");
        if negate {
            self.i += 3; // "Not"
            self.skip_ws();
        }
        self.skip_ws();
        match self.s.get(self.i)? {
            b'<' => {
                self.i += 1;
                let token = self.until(b'>')?;
                Some(Condition {
                    negate,
                    token: token.to_string(),
                    etag: false,
                })
            }
            b'[' => {
                // Entity-tag condition (RFC 4918 §10.4.4): satisfied when the
                // resource's current ETag matches (strong comparison after
                // optional-quote normalization).
                self.i += 1;
                let tag = self.until(b']')?;
                Some(Condition {
                    negate,
                    token: tag.to_string(),
                    etag: true,
                })
            }
            _ => None,
        }
    }

    fn list(&mut self) -> Option<ConditionList> {
        if !self.eat(b'(') {
            return None;
        }
        let mut conditions = Vec::new();
        loop {
            self.skip_ws();
            if self.eat(b')') {
                break;
            }
            conditions.push(self.condition()?);
        }
        if conditions.is_empty() {
            return None;
        }
        Some(ConditionList { conditions })
    }
}

/// Parse an `If` header value. Returns `None` on malformed input.
#[must_use]
pub fn parse(value: &str) -> Option<IfHeader> {
    let mut p = Parser::new(value);
    let mut untagged: Vec<ConditionList> = Vec::new();
    let mut tagged: Vec<TaggedGroup> = Vec::new();

    loop {
        p.skip_ws();
        if p.i >= p.s.len() {
            break;
        }
        if p.s[p.i] == b'<' {
            // Could be a resource tag (followed by a list) or a bare token
            // in an untagged list — disambiguate by what follows the '>'.
            let save = p.i;
            p.i += 1;
            let resource = p.until(b'>')?;
            p.skip_ws();
            if p.i < p.s.len() && p.s[p.i] == b'(' {
                // Tagged group.
                let mut lists = Vec::new();
                while let Some(list) = p.list() {
                    lists.push(list);
                    p.skip_ws();
                    if p.i >= p.s.len() || p.s[p.i] != b'(' {
                        break;
                    }
                }
                if lists.is_empty() {
                    return None;
                }
                tagged.push(TaggedGroup {
                    resource: resource.to_string(),
                    lists,
                });
            } else {
                // Bare token in an untagged list.
                p.i = save;
                let list = p.list()?;
                untagged.push(list);
            }
        } else if p.s[p.i] == b'(' {
            let list = p.list()?;
            untagged.push(list);
        } else {
            return None;
        }
    }

    match (untagged.is_empty(), tagged.is_empty()) {
        (false, true) => Some(IfHeader::Untagged(untagged)),
        (true, false) => Some(IfHeader::Tagged(tagged)),
        // Mixed forms and empty headers are malformed per §10.4.2.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    const T1: &str = "opaquelocktoken:aaa";
    const T2: &str = "opaquelocktoken:bbb";

    #[test]
    fn test_untagged_single_list() {
        let h = parse(&format!("(<{T1}>)")).expect("parse");
        match &h {
            IfHeader::Untagged(lists) => assert_eq!(lists.len(), 1),
            _ => panic!("wrong form"),
        }
        assert!(h.evaluate("/r", &tokens(&[T1]), None));
        assert!(!h.evaluate("/r", &tokens(&[T2]), None));
    }

    #[test]
    fn test_untagged_and_semantics() {
        let h = parse(&format!("(<{T1}> <{T2}>)")).expect("parse");
        assert!(!h.evaluate("/r", &tokens(&[T1]), None), "AND requires both");
        assert!(h.evaluate("/r", &tokens(&[T1, T2]), None));
    }

    #[test]
    fn test_untagged_or_across_lists() {
        let h = parse(&format!("(<{T1}>) (<{T2}>)")).expect("parse");
        assert!(h.evaluate("/r", &tokens(&[T1]), None));
        assert!(h.evaluate("/r", &tokens(&[T2]), None));
        assert!(!h.evaluate("/r", &tokens(&[]), None));
    }

    #[test]
    fn test_not_condition() {
        let h = parse(&format!("(Not <{T1}>)")).expect("parse");
        // Not <t1> holds when t1 is NOT current.
        assert!(h.evaluate("/r", &tokens(&[]), None));
        assert!(!h.evaluate("/r", &tokens(&[T1]), None));
        // Not-negated tokens are not "submitted" for lock purposes.
        assert!(h.submitted_tokens().is_empty());
    }

    #[test]
    fn test_tagged_list_matching_resource() {
        let v = format!("<http://h/a> (<{T1}>) <http://h/b> (<{T2}>)");
        let h = parse(&v).expect("parse");
        match &h {
            IfHeader::Tagged(groups) => assert_eq!(groups.len(), 2),
            _ => panic!("wrong form"),
        }
        assert!(h.evaluate("/a", &tokens(&[T1]), None));
        assert!(!h.evaluate("/a", &tokens(&[T2]), None));
        assert!(h.evaluate("/b", &tokens(&[T2]), None));
        assert!(!h.evaluate("/b", &tokens(&[T1]), None));
    }

    #[test]
    fn test_tagged_no_match_passes_vacuously() {
        let v = format!("<http://h/a> (<{T1}>)");
        let h = parse(&v).expect("parse");
        assert!(h.evaluate("/other", &tokens(&[]), None));
    }

    #[test]
    fn test_etag_condition_satisfied() {
        // §10.4.4: (["abc"]) matches a resource whose ETag is abc (quote
        // normalization: the bracket content is the quoted entity-tag).
        let h = parse(r#"(["abc"])"#).unwrap();
        assert!(h.evaluate("/r", &tokens(&[]), Some("abc")));
        assert!(h.evaluate("/r", &tokens(&[]), Some(r#""abc""#)));
        assert!(!h.evaluate("/r", &tokens(&[]), Some("other")));
        assert!(!h.evaluate("/r", &tokens(&[]), None));
    }

    #[test]
    fn test_etag_double_quotes_normalized() {
        // Ferro stored etags can embed their own quotes; the header then
        // arrives double-quoted and must still match (R6 litmus fix).
        let h = parse(r#"(["abc"])"#).unwrap();
        assert!(h.evaluate("/r", &tokens(&[]), Some("\"\"abc\"\"")));
    }

    #[test]
    fn test_not_etag_condition() {
        let h = parse(r#"(Not ["stale"])"#).unwrap();
        assert!(h.evaluate("/r", &tokens(&[]), Some("current")));
        assert!(!h.evaluate("/r", &tokens(&[]), Some("stale")));
    }

    #[test]
    fn test_mixed_lock_token_and_etag_list() {
        // AND of a lock token and an ETag: both must hold.
        let h = parse(&format!(r#"(<{T1}> ["abc"])"#)).unwrap();
        assert!(h.evaluate("/r", &tokens(&[T1]), Some("abc")));
        assert!(!h.evaluate("/r", &tokens(&[]), Some("abc")), "token missing");
        assert!(!h.evaluate("/r", &tokens(&[T1]), Some("zzz")), "etag stale");
    }

    #[test]
    fn test_submitted_tokens_exclude_etags() {
        let h = parse(&format!(r#"(Not <{T1}> <{T2}> ["abc"])"#)).unwrap();
        assert_eq!(h.submitted_tokens(), vec![T2.to_string()]);
    }

    #[test]
    fn test_submitted_tokens_ignores_negated() {
        let v = format!("(<{T1}> Not <{T2}>)");
        let h = parse(&v).expect("parse");
        assert_eq!(h.submitted_tokens(), tokens(&[T1]));
    }

    #[test]
    fn test_whitespace_tolerant() {
        let h = parse(&format!("  (  <{T1}>   <{T2}> )  ")).expect("parse");
        assert!(h.evaluate("/r", &tokens(&[T1, T2]), None));
    }

    #[test]
    fn test_malformed_rejected() {
        assert!(parse("").is_none());
        assert!(parse("()").is_none());
        assert!(parse("(<").is_none());
        assert!(parse("<http://h/a>").is_none()); // tag without list
        assert!(parse("(<t1>) <http://h/a>").is_none()); // mixed forms
    }
}
