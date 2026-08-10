//! Known- and unknown-code Problem rendering without trusting remote text.

use dispatch_diagnostics::JobNotFound;
use recourse::{
    catalog::{Catalog, CatalogDiagnostic, CatalogSpec},
    client::{Classification, ReceivedProblem, escape_terminal},
};

use crate::{
    RenderError,
    field::{
        append_field, append_issues, append_raw, append_suggestions, append_type_issue, code_text,
    },
};

/// Renders one tolerant Problem using local definitions when available.
pub fn render_problem<C: CatalogSpec>(
    catalog: &Catalog<C>,
    problem: &ReceivedProblem,
) -> Result<String, RenderError> {
    let mut rendered = match catalog.classify(problem) {
        Classification::Known(definition) => render_known(definition, problem),
        Classification::Unknown => render_unknown(problem),
    };
    append_raw(&mut rendered, problem.raw())?;
    Ok(rendered)
}

fn render_known(definition: &CatalogDiagnostic, problem: &ReceivedProblem) -> String {
    let mut rendered = format!(
        "{} — {}\nHTTP {}\n",
        definition.code(),
        escape_terminal(definition.title()),
        problem.transport_status().as_u16()
    );
    append_context(&mut rendered, problem);
    append_type_issue(&mut rendered, definition, problem.type_uri());
    append_typed_job(&mut rendered, problem);
    rendered
}

fn render_unknown(problem: &ReceivedProblem) -> String {
    let mut rendered = format!(
        "Unknown diagnostic {}\nHTTP {}\n",
        escape_terminal(&code_text(problem.code())),
        problem.transport_status().as_u16()
    );
    append_field(&mut rendered, "Title", problem.title());
    append_context(&mut rendered, problem);
    rendered
}

fn append_context(rendered: &mut String, problem: &ReceivedProblem) {
    append_field(rendered, "Detail", problem.detail());
    append_field(rendered, "Instance", problem.instance());
    append_suggestions(rendered, problem.suggestions());
    append_issues(rendered, problem.protocol_issues());
}

/// Reads the one known code whose evidence this client acts on.
///
/// Typed access is deliberately best effort: an old client must keep rendering
/// a newer or malformed document, so a decoding failure becomes another
/// protocol issue rather than a rendering failure. A type mismatch is already
/// reported by the type-URI check, so it is not repeated here.
fn append_typed_job(rendered: &mut String, problem: &ReceivedProblem) {
    let Ok(Some(typed)) = problem.try_as::<JobNotFound>() else {
        return;
    };
    match typed.evidence() {
        Ok(evidence) => append_field(rendered, "Job", Some(evidence.job_id.as_str())),
        Err(error) => {
            // The error already names the typed-evidence step it failed at, so
            // this only supplies the shared issue label.
            rendered.push_str("Protocol issue: ");
            rendered.push_str(&escape_terminal(&error.to_string()));
            rendered.push('\n');
        }
    }
}
