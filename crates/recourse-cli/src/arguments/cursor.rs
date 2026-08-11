//! Bounds-checked option cursor that preserves opaque path values.

use std::{ffi::OsString, path::PathBuf};

use recourse::catalog::CodeNumber;

use super::{ArgumentError, OutputFormat};

pub(super) struct Cursor<'a> {
    values: &'a [OsString],
    index: usize,
}

impl<'a> Cursor<'a> {
    pub(super) const fn new(values: &'a [OsString]) -> Self {
        Self { values, index: 0 }
    }

    pub(super) fn option(&mut self) -> Result<Option<String>, ArgumentError> {
        let Some(value) = self.values.get(self.index) else {
            return Ok(None);
        };
        self.index += 1;
        value
            .to_str()
            .map(str::to_owned)
            .map(Some)
            .ok_or(ArgumentError::NonUtf8Option)
    }

    pub(super) fn path(&mut self, option: &'static str) -> Result<PathBuf, ArgumentError> {
        let value = self
            .values
            .get(self.index)
            .ok_or(ArgumentError::MissingValue(option))?;
        self.index += 1;
        Ok(PathBuf::from(value))
    }

    pub(super) fn format(&mut self) -> Result<OutputFormat, ArgumentError> {
        match self.text("--format")?.as_str() {
            "human" => Ok(OutputFormat::Human),
            "json" => Ok(OutputFormat::Json),
            value => Err(ArgumentError::InvalidFormat(value.to_owned())),
        }
    }

    pub(super) fn number(&mut self) -> Result<CodeNumber, ArgumentError> {
        let value = self.text("--number")?;
        let number = value
            .parse::<u32>()
            .map_err(|_| ArgumentError::InvalidNumber(value))?;
        CodeNumber::try_new(number).map_err(|_| ArgumentError::InvalidNumber(number.to_string()))
    }

    pub(super) fn text(&mut self, option: &'static str) -> Result<String, ArgumentError> {
        let value = self
            .values
            .get(self.index)
            .ok_or(ArgumentError::MissingValue(option))?;
        self.index += 1;
        value
            .to_str()
            .map(str::to_owned)
            .ok_or(ArgumentError::NonUtf8Value(option))
    }
}
