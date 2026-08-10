//! Application-owned catalog namespace declaration.

/// Declares the stable namespace shared by a diagnostic catalog.
///
/// Implementations are marker types. Catalog construction validates every
/// constant before deriving public codes or type URIs.
pub trait CatalogSpec: Send + Sync + 'static {
    /// Stable lowercase kebab-case catalog name.
    const NAME: &'static str;

    /// Stable uppercase prefix used by every catalog code.
    const PREFIX: &'static str;

    /// Absolute URI base, ending in `/`, used to derive problem type URIs.
    const TYPE_BASE: &'static str;
}
