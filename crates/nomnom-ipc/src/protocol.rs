//! Message types. Parsing is strict: unknown message types, unknown fields, a wrong version and
//! oversized values are all rejected, so the app only ever acts on what it understands.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const VERSION: u32 = 1;
pub const MAX_URL: usize = 2048;
pub const MAX_MEDIA: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresetKind {
    Video,
    Audio,
}

/// What to download. Anything left out comes from the user's defaults in NOMNOM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    pub kind: PresetKind,
    #[serde(default)]
    pub format: Option<String>,
    /// `None` means best available.
    #[serde(default)]
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Body {
    Ping,
    Enqueue {
        url: String,
        preset: Option<Preset>,
    },
    /// Images (and similar) that the site serves as plain files, from a known post.
    EnqueueDirect {
        source: String,
        media: Vec<String>,
    },
    OpenInApp {
        url: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: String,
    pub body: Body,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtoError {
    #[error("not valid JSON")]
    NotJson,
    #[error("the message must be an object")]
    NotAnObject,
    #[error("unsupported protocol version")]
    BadVersion,
    #[error("missing or invalid field: {0}")]
    Field(&'static str),
    #[error("unknown message type")]
    UnknownType,
    #[error("unexpected field: {0}")]
    UnknownField(String),
}

fn string_field(
    obj: &Map<String, Value>,
    key: &'static str,
    max: usize,
) -> Result<String, ProtoError> {
    match obj.get(key) {
        Some(Value::String(s)) if !s.is_empty() && s.len() <= max => Ok(s.clone()),
        _ => Err(ProtoError::Field(key)),
    }
}

fn only_keys(obj: &Map<String, Value>, allowed: &[&str]) -> Result<(), ProtoError> {
    match obj.keys().find(|k| !allowed.contains(&k.as_str())) {
        Some(k) => Err(ProtoError::UnknownField(k.clone())),
        None => Ok(()),
    }
}

pub fn parse_request(bytes: &[u8]) -> Result<Request, ProtoError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| ProtoError::NotJson)?;
    let Value::Object(obj) = value else {
        return Err(ProtoError::NotAnObject);
    };
    if obj.get("v").and_then(Value::as_u64) != Some(VERSION as u64) {
        return Err(ProtoError::BadVersion);
    }
    let id = string_field(&obj, "id", 64)?;
    let kind = string_field(&obj, "type", 32)?;
    let body = match kind.as_str() {
        "ping" => {
            only_keys(&obj, &["v", "id", "type"])?;
            Body::Ping
        }
        "enqueue" => {
            only_keys(&obj, &["v", "id", "type", "url", "preset"])?;
            let preset = match obj.get("preset") {
                None | Some(Value::Null) => None,
                Some(p) => Some(
                    serde_json::from_value::<Preset>(p.clone())
                        .map_err(|_| ProtoError::Field("preset"))?,
                ),
            };
            Body::Enqueue {
                url: string_field(&obj, "url", MAX_URL)?,
                preset,
            }
        }
        "enqueue_direct" => {
            only_keys(&obj, &["v", "id", "type", "source", "media"])?;
            let media = match obj.get("media") {
                Some(Value::Array(items)) if !items.is_empty() && items.len() <= MAX_MEDIA => items
                    .iter()
                    .map(|v| match v {
                        Value::String(s) if !s.is_empty() && s.len() <= MAX_URL => Ok(s.clone()),
                        _ => Err(ProtoError::Field("media")),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => return Err(ProtoError::Field("media")),
            };
            Body::EnqueueDirect {
                source: string_field(&obj, "source", MAX_URL)?,
                media,
            }
        }
        "open_in_app" => {
            only_keys(&obj, &["v", "id", "type", "url"])?;
            Body::OpenInApp {
                url: string_field(&obj, "url", MAX_URL)?,
            }
        }
        _ => return Err(ProtoError::UnknownType),
    };
    Ok(Request { id, body })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unsupported,
    NotRunning,
    Invalid,
    RateLimited,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub v: u32,
    pub id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duplicate: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
}

impl Response {
    pub fn ok(id: impl Into<String>) -> Self {
        Response {
            v: VERSION,
            id: id.into(),
            ok: true,
            app: None,
            tasks: None,
            duplicate: None,
            error: None,
        }
    }

    pub fn err(id: impl Into<String>, code: ErrorCode, message: impl Into<String>) -> Self {
        Response {
            v: VERSION,
            id: id.into(),
            ok: false,
            app: None,
            tasks: None,
            duplicate: None,
            error: Some(ErrorBody {
                code,
                message: message.into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Request, ProtoError> {
        parse_request(s.as_bytes())
    }

    #[test]
    fn parses_each_message_type() {
        assert_eq!(
            p(r#"{"v":1,"id":"a","type":"ping"}"#).unwrap().body,
            Body::Ping
        );
        let r = p(r#"{"v":1,"id":"a","type":"enqueue","url":"https://youtu.be/x","preset":{"kind":"audio","format":"mp3"}}"#).unwrap();
        assert_eq!(
            r.body,
            Body::Enqueue {
                url: "https://youtu.be/x".into(),
                preset: Some(Preset {
                    kind: PresetKind::Audio,
                    format: Some("mp3".into()),
                    height: None
                })
            }
        );
        let r = p(r#"{"v":1,"id":"a","type":"enqueue_direct","source":"https://x.com/a/status/1","media":["https://pbs.twimg.com/media/a.jpg"]}"#).unwrap();
        assert!(matches!(r.body, Body::EnqueueDirect { ref media, .. } if media.len() == 1));
        assert!(matches!(
            p(r#"{"v":1,"id":"a","type":"open_in_app","url":"https://youtu.be/x"}"#)
                .unwrap()
                .body,
            Body::OpenInApp { .. }
        ));
    }

    #[test]
    fn enqueue_without_preset_uses_the_users_defaults() {
        let r = p(r#"{"v":1,"id":"a","type":"enqueue","url":"https://youtu.be/x"}"#).unwrap();
        assert_eq!(
            r.body,
            Body::Enqueue {
                url: "https://youtu.be/x".into(),
                preset: None
            }
        );
    }

    #[test]
    fn rejects_malformed_and_hostile_input() {
        assert_eq!(p("not json"), Err(ProtoError::NotJson));
        assert_eq!(p("[1,2]"), Err(ProtoError::NotAnObject));
        assert_eq!(
            p(r#"{"v":2,"id":"a","type":"ping"}"#),
            Err(ProtoError::BadVersion)
        );
        assert_eq!(
            p(r#"{"id":"a","type":"ping"}"#),
            Err(ProtoError::BadVersion)
        );
        assert_eq!(p(r#"{"v":1,"type":"ping"}"#), Err(ProtoError::Field("id")));
        assert_eq!(
            p(r#"{"v":1,"id":"a","type":"run_command"}"#),
            Err(ProtoError::UnknownType)
        );
        assert!(matches!(
            p(r#"{"v":1,"id":"a","type":"ping","extra":1}"#),
            Err(ProtoError::UnknownField(_))
        ));
        assert!(matches!(
            p(r#"{"v":1,"id":"a","type":"enqueue","url":"https://a.test/","exec":"calc"}"#),
            Err(ProtoError::UnknownField(_))
        ));
        assert_eq!(
            p(r#"{"v":1,"id":"a","type":"enqueue","url":""}"#),
            Err(ProtoError::Field("url"))
        );
        assert_eq!(
            p(r#"{"v":1,"id":"a","type":"enqueue","url":5}"#),
            Err(ProtoError::Field("url"))
        );
        assert_eq!(
            p(r#"{"v":1,"id":"a","type":"enqueue_direct","source":"https://a.test/","media":[]}"#),
            Err(ProtoError::Field("media"))
        );
        assert_eq!(
            p(
                r#"{"v":1,"id":"a","type":"enqueue","url":"https://a.test/","preset":{"kind":"video","evil":1}}"#
            ),
            Err(ProtoError::Field("preset"))
        );
        assert_eq!(
            p(
                r#"{"v":1,"id":"a","type":"enqueue","url":"https://a.test/","preset":{"kind":"gif"}}"#
            ),
            Err(ProtoError::Field("preset"))
        );
    }

    #[test]
    fn rejects_oversized_values() {
        let long = "x".repeat(MAX_URL + 1);
        assert_eq!(
            p(&format!(
                r#"{{"v":1,"id":"a","type":"enqueue","url":"{long}"}}"#
            )),
            Err(ProtoError::Field("url"))
        );
        let id = "i".repeat(65);
        assert_eq!(
            p(&format!(r#"{{"v":1,"id":"{id}","type":"ping"}}"#)),
            Err(ProtoError::Field("id"))
        );
        let many: Vec<String> = (0..=MAX_MEDIA)
            .map(|i| format!("\"https://i.redd.it/{i}.jpg\""))
            .collect();
        assert_eq!(
            p(&format!(
                r#"{{"v":1,"id":"a","type":"enqueue_direct","source":"https://reddit.com/x","media":[{}]}}"#,
                many.join(",")
            )),
            Err(ProtoError::Field("media"))
        );
    }

    #[test]
    fn responses_serialize_compactly_and_round_trip() {
        let ok = serde_json::to_string(&Response::ok("a")).unwrap();
        assert_eq!(ok, r#"{"v":1,"id":"a","ok":true}"#);
        let err = Response::err("a", ErrorCode::RateLimited, "Slow down.");
        let back: Response = serde_json::from_str(&serde_json::to_string(&err).unwrap()).unwrap();
        assert_eq!(back, err);
        assert!(serde_json::to_string(&err)
            .unwrap()
            .contains("rate_limited"));
    }
}
