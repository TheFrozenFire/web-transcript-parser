//! Tooling for working with HTTP data.

pub mod commit;
pub mod context;
pub mod transcript;

pub use commit::{DefaultHttpCommitter, HttpCommit, HttpCommitError};
pub use context::{HttpContext, BodyContext, RequestContext, ResponseContext};
pub use transcript::HttpTranscript;

#[doc(hidden)]
pub use spanner::http;

pub use http::{
    parse_request, parse_response, Body, BodyContent, Header, HeaderName, HeaderValue, Method,
    Reason, Request, RequestLine, Requests, Response, Responses, Status, Target,
};

/*
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use spanner::{
        http::{parse_request, BodyContent},
        json::{
            JsonValue, JsonVisit
        }, json as json_types, Spanned
    };
    use crate::transcript::Transcript;
    use tlsn_data_fixtures::http as fixtures;
    use rangeset::{ Difference, ToRangeSet, RangeSet };

    struct JsonLiteralCollector {
        literals: Vec<JsonValue>,
    }

    impl JsonLiteralCollector {
        fn new() -> Self {
            Self { literals: Vec::new() }
        }
    }

    impl JsonVisit for JsonLiteralCollector {
        fn visit_string(&mut self, node: &json_types::String) { self.literals.push(JsonValue::String(node.clone())); }
        fn visit_number(&mut self, node: &json_types::Number) { self.literals.push(JsonValue::Number(node.clone())); }
        fn visit_bool(&mut self, node: &json_types::Bool) { self.literals.push(JsonValue::Bool(node.clone())); }
        fn visit_null(&mut self, node: &json_types::Null) { self.literals.push(JsonValue::Null(node.clone())); }
    }

    #[rstest]
    #[case::get_empty(fixtures::request::GET_EMPTY)]
    #[case::get_empty_header(fixtures::request::GET_EMPTY_HEADER)]
    #[case::get_with_header(fixtures::request::GET_WITH_HEADER)]
    #[case::post_json(fixtures::request::POST_JSON)]
    fn test_http_transcript_parse_partial_request(#[case] src: &'static [u8]) {
        let transcript = Transcript::new(src, []);
        let request = parse_request(src).unwrap();

        let mut request_ranges = request.span().to_range_set();

        request_ranges = request
            .headers
            .iter()
            .filter(|h| !matches!(h.name.as_str(), "Host" | "Content-Length" | "Content-Type"))
            .map(|h| h.value.span().to_range_set())
            .fold(request_ranges, |acc, e| acc.difference(&e));

        if let Some(body) = &request.body {
            if let BodyContent::Json(json) = &body.content {
                let mut collector = JsonLiteralCollector::new();
                collector.visit_value(json);

                request_ranges = collector
                    .literals
                    .iter()
                    .map(|e| e.to_range_set())
                    .fold(request_ranges, |acc, e| acc.difference(&e));                
            }
        }

        let partial_transcript = transcript.to_partial(request_ranges, RangeSet::default());

        HttpTranscript::parse_partial(&partial_transcript).unwrap();
    }

    #[rstest]
    #[case::ok_empty(fixtures::response::OK_EMPTY)]
    #[case::ok_empty_header(fixtures::response::OK_EMPTY_HEADER)]
    #[case::ok_text(fixtures::response::OK_TEXT)]
    #[case::ok_json(fixtures::response::OK_JSON)]
    fn test_http_transcript_parse_partial_response(#[case] src: &'static [u8]) {
        let transcript = Transcript::new(fixtures::request::GET_EMPTY, src);
        let response = parse_response(src).unwrap();

        let mut response_ranges = response.span().to_range_set();

        response_ranges = response
            .headers
            .iter()
            .filter(|h| !matches!(h.name.as_str(), "Content-Length" | "Content-Type"))
            .map(|h| h.value.span().to_range_set())
            .fold(response_ranges, |acc, e| acc.difference(&e));

        if let Some(body) = &response.body {
            if let BodyContent::Json(json) = &body.content {
                let mut collector = JsonLiteralCollector::new();
                collector.visit_value(json);

                response_ranges = collector
                    .literals
                    .iter()
                    .map(|e| e.to_range_set())
                    .fold(response_ranges, |acc, e| acc.difference(&e));
            }
        }
        
        let partial_transcript = transcript.to_partial(RangeSet::from(0..transcript.sent().len()), response_ranges);

        HttpTranscript::parse_partial(&partial_transcript).unwrap();
    }
}
*/