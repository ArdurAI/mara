//! Regex-based PII / secret redaction.
//!
//! Each [`Pack`] is a set of named patterns that get compiled at
//! load and then applied to every event's string body, completion
//! text, and string-valued attributes.  Replacement tokens use the
//! `[kind]` convention (e.g., `[email]`, `[ssh-key]`, `[ssn]`).
//!
//! The built-in `pii` pack covers a curated set of common patterns;
//! operators add custom packs via configuration in M2 follow-up.

use async_trait::async_trait;
use mara_core::error::Result;
use mara_core::policy::{Policy, PolicyContext, PolicyOutcome};
use mara_schema::{AttrValue, CompletionBody, Event, EventBody, PolicyDecisionKind, PromptBody};
use regex::Regex;

/// A compiled named regex with a fixed replacement.
#[derive(Debug)]
pub struct Rule {
    /// Operator-facing kind (`email`, `aws-access-key`, ...).
    pub kind: &'static str,
    /// Compiled regex.
    pub regex: Regex,
}

/// A bundle of redaction [`Rule`]s.
#[derive(Debug)]
pub struct Pack {
    /// Pack identifier (`builtin.pii`).
    pub name: &'static str,
    /// Pack rules.
    pub rules: Vec<Rule>,
}

impl Pack {
    /// Construct the built-in PII pack.
    ///
    /// Patterns are intentionally conservative (high precision over
    /// recall).  Operators can extend with custom packs.  Each
    /// pattern is verified at load time to be linear-time via the
    /// `regex` crate's guarantee.
    #[must_use]
    pub fn builtin_pii() -> Self {
        Self {
            name: "builtin.pii",
            rules: vec![
                Rule {
                    kind: "email",
                    regex: Regex::new(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b")
                        .expect("email regex compiles"),
                },
                Rule {
                    kind: "us-phone",
                    regex: Regex::new(
                        r"\b(?:\+?1[-.\s]?)?\(?[2-9]\d{2}\)?[-.\s]?\d{3}[-.\s]?\d{4}\b",
                    )
                    .expect("us-phone regex compiles"),
                },
                Rule {
                    kind: "us-ssn",
                    regex: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("us-ssn regex compiles"),
                },
                Rule {
                    kind: "aws-access-key",
                    regex: Regex::new(r"\b(?:AKIA|ASIA)[A-Z0-9]{16}\b")
                        .expect("aws-key regex compiles"),
                },
                Rule {
                    kind: "github-token",
                    regex: Regex::new(r"\bgh[oprsu]_[A-Za-z0-9]{36,255}\b")
                        .expect("github regex compiles"),
                },
                Rule {
                    kind: "openai-key",
                    regex: Regex::new(r"\bsk-(?:proj-)?[A-Za-z0-9\-_]{20,}\b")
                        .expect("openai regex compiles"),
                },
                Rule {
                    kind: "anthropic-key",
                    regex: Regex::new(r"\bsk-ant-[A-Za-z0-9\-_]{20,}\b")
                        .expect("anthropic regex compiles"),
                },
                Rule {
                    kind: "slack-token",
                    regex: Regex::new(r"\bxox[abprs]-[A-Za-z0-9-]{10,}\b")
                        .expect("slack regex compiles"),
                },
                Rule {
                    kind: "jwt",
                    regex: Regex::new(r"\beyJ[A-Za-z0-9_-]+\.eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b")
                        .expect("jwt regex compiles"),
                },
            ],
        }
    }

    /// Apply all rules to a string, returning the redacted form and
    /// the list of rule kinds that matched.
    pub fn redact_string(&self, s: &str) -> (String, Vec<&'static str>) {
        let mut current = s.to_owned();
        let mut hits = Vec::new();
        for rule in &self.rules {
            if rule.regex.is_match(&current) {
                hits.push(rule.kind);
                let replacement = format!("[{}]", rule.kind);
                current = rule.regex.replace_all(&current, replacement.as_str()).into_owned();
            }
        }
        (current, hits)
    }
}

/// Policy implementation that applies a [`Pack`].
pub struct RegexRedactor {
    pack: Pack,
}

impl RegexRedactor {
    /// Construct a redactor from a pack.
    #[must_use]
    pub fn new(pack: Pack) -> Self {
        Self { pack }
    }

    /// Construct a redactor with the built-in PII pack.
    #[must_use]
    pub fn builtin_pii() -> Self {
        Self::new(Pack::builtin_pii())
    }

    fn redact_event(&self, ev: &mut Event) -> Vec<&'static str> {
        let mut hits = Vec::new();

        // Redact string-valued attributes.
        for v in ev.attributes.values_mut() {
            if let AttrValue::String(s) = v {
                let (red, rule_hits) = self.pack.redact_string(s);
                *s = red;
                hits.extend(rule_hits);
            }
        }

        // Redact body if present (opt-in capture path).
        if let Some(body) = ev.body.as_mut() {
            redact_body(&self.pack, body, &mut hits);
        }

        hits
    }
}

fn redact_body(pack: &Pack, body: &mut EventBody, hits: &mut Vec<&'static str>) {
    if let Some(prompt) = body.prompt.as_mut() {
        redact_prompt(pack, prompt, hits);
    }
    if let Some(completion) = body.completion.as_mut() {
        redact_completion(pack, completion, hits);
    }
    if let Some(args) = body.tool_call.as_mut().and_then(|tc| tc.arguments_json.as_mut()) {
        let (red, rule_hits) = pack.redact_string(args);
        *args = red;
        hits.extend(rule_hits);
    }
    if let Some(content) = body.tool_result.as_mut().and_then(|tr| tr.content_json.as_mut()) {
        let (red, rule_hits) = pack.redact_string(content);
        *content = red;
        hits.extend(rule_hits);
    }
    if let Some(req) = body.raw_request.as_mut() {
        let (red, rule_hits) = pack.redact_string(req);
        *req = red;
        hits.extend(rule_hits);
    }
    if let Some(res) = body.raw_response.as_mut() {
        let (red, rule_hits) = pack.redact_string(res);
        *res = red;
        hits.extend(rule_hits);
    }
}

fn redact_prompt(pack: &Pack, prompt: &mut PromptBody, hits: &mut Vec<&'static str>) {
    for msg in &mut prompt.messages {
        let (red, rule_hits) = pack.redact_string(&msg.content);
        msg.content = red;
        hits.extend(rule_hits);
    }
}

fn redact_completion(pack: &Pack, completion: &mut CompletionBody, hits: &mut Vec<&'static str>) {
    for choice in &mut completion.choices {
        let (red, rule_hits) = pack.redact_string(&choice.message.content);
        choice.message.content = red;
        hits.extend(rule_hits);
    }
}

#[async_trait]
impl Policy for RegexRedactor {
    fn name(&self) -> &str {
        self.pack.name
    }

    async fn apply(&self, ctx: &PolicyContext, mut event: Event) -> Result<PolicyOutcome> {
        let hits = self.redact_event(&mut event);
        if !hits.is_empty() {
            let reason = format!("matched: {}", hits.join(","));
            ctx.record_decision(&mut event, PolicyDecisionKind::Redacted, Some(reason));
        } else {
            ctx.record_decision(&mut event, PolicyDecisionKind::Passthrough, None);
        }
        Ok(PolicyOutcome::pass(event))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use mara_schema::{AttrValue, EventKind};

    use super::*;

    #[tokio::test]
    async fn redacts_email_in_string_attribute() {
        let mut ev = Event::now(EventKind::Prompt, "test");
        ev.attributes.insert("user.email".into(), AttrValue::String("alice@example.com".into()));
        let r = RegexRedactor::builtin_pii();
        let out = r.apply(&PolicyContext::new("redact", "default"), ev).await.unwrap();
        match out {
            PolicyOutcome::Pass(ev) => {
                let v = ev.attributes.get("user.email").unwrap();
                if let AttrValue::String(s) = v {
                    assert_eq!(s, "[email]");
                } else {
                    panic!("expected string value");
                }
                // Decision recorded.
                assert_eq!(ev.mara.policy_decisions.len(), 1);
                assert_eq!(ev.mara.policy_decisions[0].decision, PolicyDecisionKind::Redacted);
            }
            _ => panic!("expected pass"),
        }
    }

    #[tokio::test]
    async fn redacts_openai_key_in_body_prompt() {
        let mut ev = Event::now(EventKind::Prompt, "test");
        ev.body = Some(EventBody {
            prompt: Some(PromptBody {
                messages: vec![mara_schema::Message {
                    role: "user".into(),
                    content: "my key is sk-proj-AAAAAAAAAAAAAAAAAAAAAA, dont share".into(),
                }],
            }),
            ..Default::default()
        });
        let r = RegexRedactor::builtin_pii();
        let out = r.apply(&PolicyContext::new("redact", "default"), ev).await.unwrap();
        match out {
            PolicyOutcome::Pass(ev) => {
                let prompt = ev.body.as_ref().unwrap().prompt.as_ref().unwrap();
                assert!(prompt.messages[0].content.contains("[openai-key]"));
                assert!(!prompt.messages[0].content.contains("sk-proj-"));
            }
            _ => panic!("expected pass"),
        }
    }

    #[tokio::test]
    async fn passthrough_records_decision_when_nothing_matches() {
        let ev = Event { attributes: BTreeMap::new(), ..Event::now(EventKind::System, "test") };
        let r = RegexRedactor::builtin_pii();
        let out = r.apply(&PolicyContext::new("redact", "default"), ev).await.unwrap();
        match out {
            PolicyOutcome::Pass(ev) => {
                assert_eq!(ev.mara.policy_decisions.len(), 1);
                assert_eq!(ev.mara.policy_decisions[0].decision, PolicyDecisionKind::Passthrough);
            }
            _ => panic!("expected pass"),
        }
    }
}
