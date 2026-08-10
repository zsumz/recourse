//! Conservative configurable resource limits for untrusted diagnostic JSON.

/// Resource and shape budgets applied before semantic decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeLimits {
    body_bytes: usize,
    nesting_depth: usize,
    object_properties: usize,
    array_items: usize,
    string_bytes: usize,
    suggestions: usize,
    violations: usize,
}

impl DecodeLimits {
    /// Default maximum encoded body size in bytes.
    pub const DEFAULT_MAX_BODY_BYTES: usize = 64 * 1024;
    /// Default maximum nested object or array depth.
    pub const DEFAULT_MAX_NESTING_DEPTH: usize = 32;
    /// Default maximum property count for each object.
    pub const DEFAULT_MAX_OBJECT_PROPERTIES: usize = 128;
    /// Default maximum item count for each array.
    pub const DEFAULT_MAX_ARRAY_ITEMS: usize = 128;
    /// Default maximum UTF-8 size for each key or string value.
    pub const DEFAULT_MAX_STRING_BYTES: usize = 8 * 1024;
    /// Default maximum top-level suggestion count.
    pub const DEFAULT_MAX_SUGGESTIONS: usize = 32;
    /// Default maximum validation violation count.
    pub const DEFAULT_MAX_VIOLATIONS: usize = 128;

    /// Replaces the body-byte budget.
    #[must_use]
    pub const fn with_max_body_bytes(mut self, maximum: usize) -> Self {
        self.body_bytes = maximum;
        self
    }

    /// Replaces the nesting-depth budget.
    #[must_use]
    pub const fn with_max_nesting_depth(mut self, maximum: usize) -> Self {
        self.nesting_depth = maximum;
        self
    }

    /// Replaces the per-object property budget.
    #[must_use]
    pub const fn with_max_object_properties(mut self, maximum: usize) -> Self {
        self.object_properties = maximum;
        self
    }

    /// Replaces the per-array item budget.
    #[must_use]
    pub const fn with_max_array_items(mut self, maximum: usize) -> Self {
        self.array_items = maximum;
        self
    }

    /// Replaces the per-string UTF-8 byte budget.
    #[must_use]
    pub const fn with_max_string_bytes(mut self, maximum: usize) -> Self {
        self.string_bytes = maximum;
        self
    }

    /// Replaces the suggestion-count budget.
    #[must_use]
    pub const fn with_max_suggestions(mut self, maximum: usize) -> Self {
        self.suggestions = maximum;
        self
    }

    /// Replaces the validation-violation budget.
    #[must_use]
    pub const fn with_max_violations(mut self, maximum: usize) -> Self {
        self.violations = maximum;
        self
    }

    pub(crate) const fn max_body_bytes(self) -> usize {
        self.body_bytes
    }

    pub(crate) const fn max_nesting_depth(self) -> usize {
        self.nesting_depth
    }

    pub(crate) const fn max_object_properties(self) -> usize {
        self.object_properties
    }

    pub(crate) const fn max_array_items(self) -> usize {
        self.array_items
    }

    pub(crate) const fn max_string_bytes(self) -> usize {
        self.string_bytes
    }

    pub(crate) const fn max_suggestions(self) -> usize {
        self.suggestions
    }

    pub(crate) const fn max_violations(self) -> usize {
        self.violations
    }
}

impl Default for DecodeLimits {
    fn default() -> Self {
        Self {
            body_bytes: Self::DEFAULT_MAX_BODY_BYTES,
            nesting_depth: Self::DEFAULT_MAX_NESTING_DEPTH,
            object_properties: Self::DEFAULT_MAX_OBJECT_PROPERTIES,
            array_items: Self::DEFAULT_MAX_ARRAY_ITEMS,
            string_bytes: Self::DEFAULT_MAX_STRING_BYTES,
            suggestions: Self::DEFAULT_MAX_SUGGESTIONS,
            violations: Self::DEFAULT_MAX_VIOLATIONS,
        }
    }
}
