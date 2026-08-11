//! Public evidence and stable semantic diagnostic declarations.

mod declaration;
mod evidence;
mod text;

pub use declaration::DiagnosticType;
pub use evidence::{NoEvidence, PublicEvidence};
pub use text::{DEFAULT_PUBLIC_TEXT_CHARS, PublicText, PublicTextError};

pub(crate) use text::{contains_control_character, count_characters};

#[cfg(test)]
mod declaration_test;
#[cfg(test)]
mod evidence_test;
#[cfg(test)]
mod text_test;
