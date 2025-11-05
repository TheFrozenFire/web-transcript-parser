use spanner::http::{Request, Body, BodyContent};

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

impl HttpContext {
    /// Creates a new builder.
    pub fn builder(
        transcript: PartialTranscript,
        structure: HttpTranscript,
    ) -> HttpContextBuilder {
        HttpContextBuilder::new(transcript, structure)
    }
}

/// Builder for [`HttpContext`].
pub struct HttpContextBuilder {
    transcript: PartialTranscript,
    structure: HttpTranscript,
}

impl HttpContextBuilder {
    /// Creates a new builder.
    pub fn new(
        transcript: PartialTranscript,
        structure: HttpTranscript,
    ) -> Self {
        Self { transcript, structure }
    }

    /// Enforces the body of an HTTP request or response.
    fn enforce_body(&self, structure_body: &Body, request_body: &Body) -> Result<BodyContext, Box<dyn std::error::Error>> {
        match (&structure_body.content, &request_body.content) {
            (BodyContent::Json(structure_json), BodyContent::Json(request_json)) => {
                Ok(BodyContext::Json(JsonContext::builder(structure_json.clone(), request_json.clone()).build()?))
            }

            (BodyContent::Unknown(structure_unknown), BodyContent::Unknown(request_unknown)) => {
                assert_eq!(structure_unknown.as_bytes(), request_unknown.as_bytes(), "Body content mismatch");
                Ok(BodyContext::Unknown(request_unknown.clone().to_bytes()))
            }

            _ => {
                return Err("Body type mismatch".into());
            }
        }
    }

    // Enforces the request target.
    // The request target must match the structure target.
    // If the structure target starts with "/", the request target may be an absolute URL or a relative URL
    // If the structure target does not start with "/", the request target must be a full URL that matches the structure target.
    fn enforce_request_target(&self, request: &Request, structure_request: &Request) -> Result<(), Box<dyn std::error::Error>> {
        let structure_target = structure_request.request.target.as_str();
        let request_target = request.request.target.as_str();

        let base = url::Url::parse("https://example.com")?;

        let structure_url = base.join(structure_target)?;
        let request_url = base.join(request_target)?;

        let structure_path_and_query = if let Some(query) = structure_url.query() {
            format!("{}?{}", structure_url.path(), query)
        } else {
            structure_url.path().to_string()
        };

        let request_path_and_query = if let Some(query) = request_url.query() {
            format!("{}?{}", request_url.path(), query)
        } else {
            request_url.path().to_string()
        };

        assert_eq!(request_path_and_query, structure_path_and_query, "Request target mismatch");
        if !structure_target.starts_with("/") {
            assert_eq!(request_url.host_str(), structure_url.host_str(), "Request target mismatch");
        }

        Ok(())
    }

    // Enforces the structure of the transcript.
    // The transcript must have the same number of requests and responses as the structure.
    // The request method and target must match.
    // The request and response headers exist if present, and must match values if specified.
    // The response status code must match if specified.
    // If the request or response body is JSON, the body must be valid JSON, and the body must match the structure.
    fn enforce_structure(&self, transcript: &HttpTranscript) -> Result<HttpContext, Box<dyn std::error::Error>> {
        assert_eq!(transcript.requests.len(), self.structure.requests.len(), "Request count mismatch");
        assert_eq!(transcript.responses.len(), self.structure.responses.len(), "Response count mismatch");

        let mut request_contexts: Vec<RequestContext> = Vec::new();
        let mut response_contexts: Vec<ResponseContext> = Vec::new();

        for (structure_request, request) in self.structure.requests.iter().zip(transcript.requests.iter()) {
            assert_eq!(request.request.method, structure_request.request.method, "Request method mismatch");
            
            self.enforce_request_target(request, structure_request)?;

            let mut request_context_headers = Vec::new();

            let structure_headers = structure_request.headers.iter()
                .filter(|h| !["content-length"].contains(&h.name.as_str().to_lowercase().as_str()));

            for structure_header in structure_headers {
                let header = request.headers_with_name(&structure_header.name.as_str()).next()
                    .ok_or_else(|| format!("Missing required header: {}", structure_header.name.as_str()))?;
                let header_name = header.name.as_str();
                let header_value = header.value.as_bytes();

                if structure_header.value.as_bytes().iter().any(|&b| b != b'*') {
                    assert_eq!(
                        header_value,
                        structure_header.value.as_bytes(),
                        "Header value mismatch: {}",
                        header_name,
                    );

                    request_context_headers.push((header_name.to_string(), String::from_utf8_lossy(header_value).to_string()));
                }
            }

            let request_body_context = if let Some(structure_body) = &structure_request.body {
                if let Some(request_body) = &request.body {
                    Some(self.enforce_body(structure_body, request_body)?)
                } else {
                    return Err("Request body missing".into());
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

        for (structure_response, response) in self.structure.responses.iter().zip(transcript.responses.iter()) {
            assert_eq!(response.status, structure_response.status, "Response status mismatch");

            let mut response_context_headers = Vec::new();

            let structure_headers = structure_response.headers.iter()
                .filter(|h| !["content-length"].contains(&h.name.as_str().to_lowercase().as_str()));

            for structure_header in structure_headers {
                let header = response.headers_with_name(&structure_header.name.as_str()).next()
                    .ok_or_else(|| format!("Missing required header: {}", structure_header.name.as_str()))?;
                let header_name = header.name.as_str();
                let header_value = header.value.as_bytes();

                if structure_header.value.as_bytes().iter().any(|&b| b != b'*') {
                    assert_eq!(
                        header_value,
                        structure_header.value.as_bytes(),
                        "Header value mismatch: {}",
                        header_name
                    );
                    response_context_headers.push((header_name.to_string(), String::from_utf8_lossy(header_value).to_string()));
                }
            }

            let response_body_context = if let Some(structure_body) = &structure_response.body {
                if let Some(response_body) = &response.body {
                    Some(self.enforce_body(structure_body, response_body)?)
                } else {
                    return Err("Response body missing".into());
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

    /// Builds the context.
    pub fn build(self) -> Result<HttpContext, Box<dyn std::error::Error>> {
        let transcript = HttpTranscript::parse_partial(&self.transcript)?;

        self.enforce_structure(&transcript)
    }
}

/// The context of a request.
#[derive(Debug, Serialize)]
pub struct RequestContext {
    target: String,
    #[serde(serialize_with = "serialize_method")]
    method: Method,
    headers: Vec<(String, String)>,
    body: Option<BodyContext>,
}

/// The context of a response.
#[derive(Debug, Serialize)]
pub struct ResponseContext {
    #[serde(serialize_with = "serialize_status_code")]
    status: StatusCode,
    headers: Vec<(String, String)>,
    body: Option<BodyContext>,
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

