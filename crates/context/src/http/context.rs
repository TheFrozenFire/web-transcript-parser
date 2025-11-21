use spanner::http::BodyContent;

use crate::http::HttpTranscript;
use crate::json::JsonContext;
use crate::transcript::PartialTranscript;

use http::{Method, StatusCode};
use std::str::FromStr;
use serde::{Serialize, Serializer};

/// A verifier of contextual integrity for HTTP presentations.
///
/// See the [module level documentation](crate::context) for more information.
#[derive(Debug, Serialize)]
pub struct HttpContext {
    requests: Vec<RequestContext>,
    responses: Vec<ResponseContext>,
}

pub struct HttpContextBuilder {
    transcript: PartialTranscript,
}

impl HttpContextBuilder {
    pub fn new(transcript: PartialTranscript) -> Self {
        Self { transcript }
    }

    pub fn build(self) -> Result<HttpContext, Box<dyn std::error::Error>> {
        let transcript = HttpTranscript::parse_partial(&self.transcript)?;

        let mut request_contexts: Vec<RequestContext> = Vec::new();
        let mut response_contexts: Vec<ResponseContext> = Vec::new();

        for request in transcript.requests.iter() {
            let mut request_context_headers = Vec::new();

            let headers = request.headers.iter()
                .filter(|h| !["content-length"].contains(&h.name.as_str().to_lowercase().as_str()));

            for header in headers {
                let header = request.headers_with_name(&header.name.as_str()).next()
                    .ok_or_else(|| format!("Missing required header: {}", header.name.as_str()))?;
                request_context_headers.push((header.name.as_str().to_string(), String::from_utf8_lossy(header.value.as_bytes()).to_string()));
            }

            let request_body_context = if let Some(body) = &request.body {
                match &body.content {
                    BodyContent::Json(json) => {
                        Some(BodyContext::Json(JsonContext::builder(json.clone()).build()?))
                    }
                    BodyContent::Unknown(unknown) => {
                        Some(BodyContext::Unknown(unknown.clone().to_bytes()))
                    }
                    _ => None,
                }
            } else {
                None
            };

            request_contexts.push(RequestContext {
                target: request.request.target.as_str().to_string(),
                method: Method::from_str(request.request.method.as_str()).unwrap(),
                headers: request_context_headers,
                body: request_body_context,
            });
        }

        for response in transcript.responses.iter() {
            let mut response_context_headers = Vec::new();

            for header in response.headers.iter() {
                let header = response.headers_with_name(&header.name.as_str()).next()
                    .ok_or_else(|| format!("Missing required header: {}", header.name.as_str()))?;
                response_context_headers.push((header.name.as_str().to_string(), String::from_utf8_lossy(header.value.as_bytes()).to_string()));
            }

            let response_body_context = if let Some(body) = &response.body {
                match &body.content {
                    BodyContent::Json(json) => {
                        Some(BodyContext::Json(JsonContext::builder(json.clone()).build()?))
                    }
                    BodyContent::Unknown(unknown) => {
                        Some(BodyContext::Unknown(unknown.clone().to_bytes()))
                    }
                    _ => None,
                }
            } else {
                None
            };

            response_contexts.push(ResponseContext {
                status: StatusCode::from_str(response.status.code.as_str()).unwrap(),
                headers: response_context_headers,
                body: response_body_context,
            });
        }

        Ok(HttpContext {
            requests: request_contexts,
            responses: response_contexts,
        })
    }
}

/// The context of a request.
#[derive(Debug, Serialize)]
pub struct RequestContext {
    pub(crate) target: String,
    #[serde(serialize_with = "serialize_method")]
    pub(crate) method: Method,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Option<BodyContext>,
}

/// The context of a response.
#[derive(Debug, Serialize)]
pub struct ResponseContext {
    #[serde(serialize_with = "serialize_status_code")]
    pub(crate) status: StatusCode,
    pub(crate) headers: Vec<(String, String)>,
    pub(crate) body: Option<BodyContext>,
}


/// The context of a body.
#[derive(Debug, Serialize)]
pub enum BodyContext {
    /// The body is JSON.
    Json(JsonContext),
    /// The body is unknown.
    #[serde(serialize_with = "serialize_bytes")]
    Unknown(bytes::Bytes),
}

// Serialization function for http::Method
fn serialize_method<S>(method: &Method, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    method.as_str().serialize(serializer)
}

// Serialization function for http::StatusCode
fn serialize_status_code<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    status.as_u16().serialize(serializer)
}

// Serialization function for bytes::Bytes
fn serialize_bytes<S>(bytes: &bytes::Bytes, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    bytes.as_ref().serialize(serializer)
}

