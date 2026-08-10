//! Validation and deterministic compilation of API operation problem sets.

use std::collections::{BTreeMap, BTreeSet};

use crate::catalog::{
    CatalogIssue, CatalogSpec, Code, CodeNumber, ProblemSet, valid_problem_set_id,
};

use super::registration::Registration;

pub(super) fn validate_problem_sets<C: CatalogSpec>(
    problem_sets: Vec<ProblemSet<C>>,
    registrations: &BTreeMap<CodeNumber, Registration>,
    issues: &mut Vec<CatalogIssue>,
) -> BTreeMap<String, Vec<CodeNumber>> {
    let mut validated = BTreeMap::new();
    for problem_set in problem_sets {
        if !valid_problem_set_id(&problem_set.id) {
            issues.push(CatalogIssue::InvalidProblemSetId {
                value: problem_set.id,
            });
            continue;
        }
        if validated.contains_key(&problem_set.id) {
            issues.push(CatalogIssue::DuplicateProblemSetId { id: problem_set.id });
            continue;
        }
        let members = validate_members(&problem_set, registrations, issues);
        validated.insert(problem_set.id, members);
    }
    validated
}

fn validate_members<C: CatalogSpec>(
    problem_set: &ProblemSet<C>,
    registrations: &BTreeMap<CodeNumber, Registration>,
    issues: &mut Vec<CatalogIssue>,
) -> Vec<CodeNumber> {
    let mut unique = BTreeSet::new();
    for member in &problem_set.members {
        if !unique.insert(member.number) {
            issues.push(CatalogIssue::DuplicateProblemSetMember {
                problem_set: problem_set.id.clone(),
                number: member.number,
            });
            continue;
        }
        let registered = registrations
            .get(&member.number)
            .is_some_and(|value| value.type_id == member.type_id && value.http.is_some());
        if !registered {
            issues.push(CatalogIssue::UnregisteredProblemSetMember {
                problem_set: problem_set.id.clone(),
                number: member.number,
            });
        }
    }
    unique.into_iter().collect()
}

pub(super) fn compile_problem_sets(
    prefix: &str,
    problem_sets: BTreeMap<String, Vec<CodeNumber>>,
) -> BTreeMap<String, Vec<Code>> {
    problem_sets
        .into_iter()
        .map(|(id, members)| {
            let codes = members
                .into_iter()
                .filter_map(|number| Code::new(prefix, number).ok())
                .collect();
            (id, codes)
        })
        .collect()
}
