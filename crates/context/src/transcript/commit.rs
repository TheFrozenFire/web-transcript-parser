use std::fmt;

use rangeset::ToRangeSet;

use crate::transcript::Direction;

pub trait TranscriptCommitmentBuilder {
    /// Adds a commitment
    ///
    /// # Arguments
    ///
    /// * `ranges` - The ranges of the commitment.
    /// * `direction` - The direction of the transcript.
    fn commit(
        &mut self,
        ranges: &dyn ToRangeSet<usize>,
        direction: Direction,
    ) -> Result<&mut Self, TranscriptCommitmentBuilderError>;

    fn build(self) -> Result<Box<dyn TranscriptCommitment>, TranscriptCommitmentBuilderError>;
}

pub trait TranscriptCommitment {
    /// Returns whether the builder has a commitment for the given direction and
    /// range.
    ///
    /// # Arguments
    ///
    /// * `direction` - The direction of the transcript.
    /// * `range` - The range of the commitment.
    fn contains(
        &self,
        ranges: &dyn ToRangeSet<usize>,
        direction: Direction,
    ) -> bool;
}

/// Error for [`TranscriptCommitmentBuilder`].
#[derive(Debug, thiserror::Error)]
pub struct TranscriptCommitmentBuilderError {
    kind: ErrorKind,
    source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl TranscriptCommitmentBuilderError {
    fn new<E>(kind: ErrorKind, source: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Send + Sync>>,
    {
        Self {
            kind,
            source: Some(source.into()),
        }
    }
}

#[derive(Debug)]
enum ErrorKind {
    Index,
}

impl fmt::Display for TranscriptCommitmentBuilderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ErrorKind::Index => f.write_str("index error")?,
        }

        if let Some(source) = &self.source {
            write!(f, " caused by: {source}")?;
        }

        Ok(())
    }
}