//! Parallel Rust parsing preserves deterministic graph inputs.

use tpt_weave_core::RepositoryId;
use tpt_weave_rust::{FileInput, ParseFileInput, parse_file, parse_files};

#[test]
fn parallel_parser_matches_sequential_results_and_order() {
    let repository = RepositoryId::new("parallel-test");
    let inputs = vec![
        ParseFileInput {
            repository: repository.clone(),
            package: "demo".to_string(),
            path: "src/lib.rs".to_string(),
            module_prefix: Vec::new(),
            source: "pub fn beta() {}\n".to_string(),
        },
        ParseFileInput {
            repository: repository.clone(),
            package: "demo".to_string(),
            path: "src/alpha.rs".to_string(),
            module_prefix: Vec::new(),
            source: "pub fn alpha() {}\n".to_string(),
        },
        ParseFileInput {
            repository: repository.clone(),
            package: "demo".to_string(),
            path: "src/broken.rs".to_string(),
            module_prefix: Vec::new(),
            source: "pub fn broken( {\n".to_string(),
        },
    ];
    let mut expected: Vec<_> = inputs
        .iter()
        .filter_map(|owned| {
            let input = FileInput {
                repository: &owned.repository,
                package: &owned.package,
                path: &owned.path,
                module_prefix: &[],
            };
            parse_file(&input, &owned.source)
                .ok()
                .map(|file| (owned.path.clone(), file))
        })
        .collect();
    expected.sort_by(|left, right| left.0.cmp(&right.0));
    let actual = parse_files(inputs.clone());

    assert_eq!(
        actual
            .iter()
            .map(|parsed| (parsed.file.path.clone(), parsed.file.clone()))
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(parse_files(inputs), actual);
}
