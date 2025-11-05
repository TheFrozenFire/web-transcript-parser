use bytes::Bytes;
pub use commit::{DefaultHttpCommitter, HttpCommit, HttpCommitError};
pub use context::{HttpContext, BodyContext, RequestContext, ResponseContext};

#[doc(hidden)]
pub use spanner::http;
use spanner::json::JsonValue;

use spanner::http::{Request, Response, Requests, Responses, BodyContent};

use crate::transcript::{Transcript, PartialTranscript, Direction, TranscriptCommitmentBuilder, TranscriptCommitmentBuilderError};

/// The kind of HTTP message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageKind {
    /// An HTTP request.
    Request,
    /// An HTTP response.
    Response,
}

/// An HTTP transcript.
#[derive(Debug)]
pub struct HttpTranscript<C: TranscriptCommitmentBuilder> {
    /// The requests sent to the server.
    pub requests: Vec<Request>,
    /// The responses received from the server.
    pub responses: Vec<Response>,
}

impl<C: TranscriptCommitmentBuilder> HttpTranscript<C> {
    /// Parses the HTTP transcript from the provided transcripts.
    pub fn parse(transcript: &Transcript) -> Result<Self, spanner::ParseError> {
        let requests = Requests::new(Bytes::copy_from_slice(transcript.sent()))
            .collect::<Result<Vec<_>, _>>()?;
        let responses = Responses::new(Bytes::copy_from_slice(transcript.received()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            requests,
            responses,
        })
    }

    /// Parses the HTTP transcript from the provided partial transcript,
    /// setting all unauthenticated data to null bytes.
    pub fn parse_partial(transcript: &PartialTranscript) -> Result<Self, spanner::ParseError> {
        let mut parseable = transcript.clone();
        parseable.set_unauthed(b'*');

        let requests = Requests::new(Bytes::copy_from_slice(parseable.sent_unsafe()))
            .collect::<Result<Vec<_>, _>>()?;
        let responses = Responses::new(Bytes::copy_from_slice(parseable.received_unsafe()))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            requests,
            responses,
        })
    }

    fn reveal_json_structure(&self, builder: &mut C, direction: Direction, json: &JsonValue) -> Result<(), TranscriptCommitmentBuilderError> {
        match json {
            JsonValue::Object(object) => {
                // Reveal the object structure without its pairs
                match direction {
                    Direction::Sent => {
                        builder.commit(&object.without_pairs(), direction)?;
                    }
                    Direction::Received => {
                        builder.commit(&object.without_pairs(), direction)?;
                    }
                }
    
                // Reveal each key-value pair structure
                for keyvalue in &object.elems {
                    match direction {
                        Direction::Sent => {
                            builder.commit(&keyvalue.without_value(), direction)?;
                        }
                        Direction::Received => {
                            builder.commit(&keyvalue.without_value(), direction)?;
                        }
                    }
    
                    // Recursively reveal the value's structure
                    self.reveal_json_structure(builder, direction, &keyvalue.value)?;
                }
            }
            JsonValue::Array(array) => {
                // Reveal array structure
                match direction {
                    Direction::Sent => {
                        builder.commit(&array.without_values(), direction)?;
                        builder.commit(&array.separators(), direction)?;
                    }
                    Direction::Received => {
                        builder.commit(&array.without_values(), direction)?;
                        builder.commit(&array.separators(), direction)?;
                    }
                }
                
                for value in &array.elems {
                    self.reveal_json_structure(builder, direction, value)?;
                }
            }
            _ => {} // For primitive values, no structure to reveal
        }

        Ok(())
    }

    /// Reveals the structure of the HTTP transcript.
    pub fn reveal_structure(&self, builder: &mut C) -> Result<(), TranscriptCommitmentBuilderError> {
        for request in &self.requests {
            builder.commit(&request.without_data(), Direction::Sent)?;
            builder.commit(&request.request.target, Direction::Sent)?;
            
            for header in &request.headers {
                builder.commit(&header.without_value(), Direction::Sent)?;
            }

            for header_name in ["host", "content-length", "content-type", "transfer-encoding"] {
                if let Some(header) = request.headers_with_name(header_name).next() {
                    builder.commit(header, Direction::Sent)?;
                }
            }

            if let Some(body) = &request.body {
                match &body.content {
                    BodyContent::Json(json) => {
                        self.reveal_json_structure(builder, Direction::Sent, json)?;
                    }
                    
                    BodyContent::Unknown(unknown) => {
                        builder.commit(unknown, Direction::Sent)?;
                    }

                    _ => {}
                }
            }
        }

        for response in &self.responses {
            builder.commit(&response.without_data(), Direction::Received)?;

            for header in &response.headers {
                match header.name.as_str().to_lowercase().as_str() {
                    "host" | "content-length" | "content-type" | "transfer-encoding" => {
                        builder.commit(header, Direction::Received)?;
                    }
                    _ => {
                        builder.commit(&header.without_value(), Direction::Received)?;
                    }
                }
            }

            if let Some(body) = &response.body {
                match &body.content {
                    BodyContent::Json(json) => {
                        self.reveal_json_structure(builder, Direction::Received, json)?;
                    }

                    BodyContent::Unknown(unknown) => {
                        builder.commit(unknown, Direction::Received)?;
                    }

                    _ => {}
                }
            }

            if let Some(boundaries) = &response.boundaries {
                for boundary in boundaries {
                    builder.commit(boundary, Direction::Received)?;
                }
            }

            if let Some(trailers) = &response.trailers {
                for trailer in trailers {
                    builder.commit(&trailer.without_value(), Direction::Received)?;
                }
            }
        }

        Ok(())
    }
}

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