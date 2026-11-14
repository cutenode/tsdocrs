//! Properties of tsdoc.json loading: what a file sets up, the files it extends, why one can't be
//! used, and which one a source file gets.

use std::{fs, path::Path};

use hegel::{TestCase, generators as gs};
use serde_json::{Map, Value, json};
use tsdocrs::{TSDocConfiguration, TSDocMessageId, TSDocTagDefinition, TSDocTagSyntaxKind, config_file};

fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Writes a tsdoc.json with `keys` and its `$schema` in `dir`, and gives its path.
fn tsdoc_json(dir: &Path, keys: &Map<String, Value>) -> std::path::PathBuf {
    let mut file = keys.clone();
    file.insert("$schema".into(), json!(config_file::SCHEMA));
    let path = dir.join(config_file::FILE_NAME);
    fs::write(&path, Value::Object(file).to_string()).unwrap();
    path
}

/// A tag definition as a tsdoc.json writes it, and as the parser takes it.
fn definition(tc: &TestCase, name: &str) -> (Value, TSDocTagDefinition) {
    let (kind, word) = tc.draw(gs::sampled_from(vec![
        (TSDocTagSyntaxKind::BlockTag, "block"),
        (TSDocTagSyntaxKind::InlineTag, "inline"),
        (TSDocTagSyntaxKind::ModifierTag, "modifier"),
    ]));
    let allow_multiple = tc.draw(gs::booleans());
    let json = json!({ "tagName": name, "syntaxKind": word, "allowMultiple": allow_multiple });
    (json, TSDocTagDefinition::new(name, kind, allow_multiple).unwrap())
}

/// A tsdoc.json sets up the parser the way `@microsoft/tsdoc-config` does: the files it extends
/// (a path, or a file in a package) first, then its own tag definitions, `supportForTags` and HTML
/// elements over theirs, and its `noStandardTags` or else theirs. Comments are fine in it. The
/// files it read come back too.
#[hegel::test(test_cases = 50)]
fn tsdoc_json_sets_up_the_parser(tc: TestCase) {
    let dir = tempfile::tempdir().unwrap();
    let (mut base, mut top) = (Map::new(), Map::new());
    let base_standard = tc.draw(gs::optional(gs::booleans()));
    let top_standard = tc.draw(gs::optional(gs::booleans()));
    let (base_file, specifier) = if tc.draw(gs::booleans()) {
        ("base/tsdoc.json", "./base/tsdoc.json")
    } else {
        ("node_modules/shared/tsdoc.json", "shared/tsdoc.json")
    };
    let extends = tc.draw(gs::booleans());
    // The base file is there either way, and counts only when the top one extends it.
    if let Some(no) = base_standard {
        base.insert("noStandardTags".into(), json!(no));
    }
    let standard = !top_standard.or(base_standard.filter(|_| extends)).unwrap_or(false);
    let mut wanted = TSDocConfiguration::new();
    wanted.clear(!standard);
    if extends {
        let (json, custom) = definition(&tc, "@custom");
        base.insert("tagDefinitions".into(), json!([json]));
        wanted.add_tag_definition(custom).unwrap();
        if tc.draw(gs::booleans()) {
            base.insert("supportedHtmlElements".into(), json!(["b", "my-el"]));
            wanted.set_supported_html_elements(["b".into(), "my-el".into()]);
        }
        top.insert("extends".into(), json!([specifier]));
    }
    if let Some(no) = top_standard {
        top.insert("noStandardTags".into(), json!(no));
    }
    let (json, mine) = definition(&tc, "@mine");
    top.insert("tagDefinitions".into(), json!([json]));
    wanted.add_tag_definition(mine).unwrap();
    let mut candidates = vec!["@MINE"];
    if extends {
        candidates.push("@custom");
    }
    if standard {
        candidates.push("@remarks");
    }
    let mut support = Map::new();
    for name in tc.draw(gs::vecs(gs::sampled_from(candidates)).max_size(4)) {
        let supported = tc.draw(gs::booleans());
        support.insert(name.into(), json!(supported));
    }
    for (name, supported) in &support {
        wanted.set_support_for_tag(name, supported == &json!(true)).unwrap();
    }
    if !support.is_empty() {
        top.insert("supportForTags".into(), Value::Object(support));
    }
    if tc.draw(gs::booleans()) {
        top.insert("supportedHtmlElements".into(), json!(["i"]));
        wanted.set_supported_html_elements(["i".into()]);
    }
    if let Some(report) = tc.draw(gs::optional(gs::booleans())) {
        top.insert("reportUnsupportedHtmlElements".into(), json!(report));
        wanted.validation.report_unsupported_html_elements = report;
    }
    base.insert("$schema".into(), json!(config_file::SCHEMA));
    write(dir.path(), base_file, &format!("// shared\n{}", Value::Object(base)));
    let path = tsdoc_json(dir.path(), &top);
    let (configuration, sources) = config_file::load(&path).unwrap();
    assert_eq!(configuration, wanted);
    let mut read = vec![path];
    if extends {
        read.push(dir.path().join(base_file));
    }
    // The file it extends comes back as the resolver names it: by its real path, which on
    // Windows can differ from the temporary folder's in form (a short name, no `\\?\`).
    let real = |paths: &[std::path::PathBuf]| paths.iter().map(|p| fs::canonicalize(p).unwrap()).collect::<Vec<_>>();
    assert_eq!(real(&sources), real(&read));
}

/// A tsdoc.json that can't be used fails with TSDoc's message id for why, naming the file: it
/// isn't there or isn't a file, isn't JSON (a stray `/` included), has another `$schema`, a key
/// tsdoc-config doesn't know, an HTML element name that isn't one, a tag name that isn't one, a
/// tag defined twice (in the file, over a file it extends, or over a standard tag), a supported tag
/// nobody defines, or extends a file that isn't there, one that can't be used, or itself.
#[hegel::test(test_cases = 3)]
fn tsdoc_json_errors_say_why(tc: TestCase) {
    use TSDocMessageId as Id;
    let schema = json!(config_file::SCHEMA);
    let tag = |name: &str| json!({ "tagName": name, "syntaxKind": "block" });
    let file = |value: Value| value.to_string();
    let cases: Vec<(Vec<(&str, String)>, Id)> = vec![
        (vec![], Id::ConfigFileNotFound),
        (vec![("tsdoc.json/x", String::new())], Id::ConfigFileNotFound),
        (vec![("tsdoc.json", "{ / }".into())], Id::ConfigInvalidJson),
        (vec![("tsdoc.json", "{".into())], Id::ConfigInvalidJson),
        (vec![("tsdoc.json", "{}".into())], Id::ConfigFileUnsupportedSchema),
        (vec![("tsdoc.json", file(json!({ "$schema": schema, "x": 1 })))], Id::ConfigFileSchemaError),
        (
            vec![("tsdoc.json", file(json!({ "$schema": schema, "supportedHtmlElements": ["b c"] })))],
            Id::ConfigFileSchemaError,
        ),
        (
            vec![("tsdoc.json", file(json!({ "$schema": schema, "supportedHtmlElements": [""] })))],
            Id::ConfigFileSchemaError,
        ),
        (
            vec![("tsdoc.json", file(json!({ "$schema": schema, "tagDefinitions": [tag("custom")] })))],
            Id::ConfigFileInvalidTagName,
        ),
        (
            vec![("tsdoc.json", file(json!({ "$schema": schema, "tagDefinitions": [tag("@a"), tag("@A")] })))],
            Id::ConfigFileDuplicateTagName,
        ),
        (
            vec![
                ("base.json", file(json!({ "$schema": schema, "tagDefinitions": [tag("@a")] }))),
                (
                    "tsdoc.json",
                    file(json!({ "$schema": schema, "extends": ["./base.json"], "tagDefinitions": [tag("@a")] })),
                ),
            ],
            Id::ConfigFileDuplicateTagName,
        ),
        (
            vec![("tsdoc.json", file(json!({ "$schema": schema, "tagDefinitions": [tag("@remarks")] })))],
            Id::ConfigFileDuplicateTagName,
        ),
        (
            vec![("tsdoc.json", file(json!({ "$schema": schema, "supportForTags": { "@nope": true } })))],
            Id::ConfigFileUndefinedTag,
        ),
        (
            vec![("tsdoc.json", file(json!({ "$schema": schema, "extends": ["./missing.json"] })))],
            Id::ConfigFileUnresolvedExtends,
        ),
        (
            vec![
                ("base.json", "{}".into()),
                ("tsdoc.json", file(json!({ "$schema": schema, "extends": ["./base.json"] }))),
            ],
            Id::ConfigFileUnsupportedSchema,
        ),
        (
            vec![
                ("base.json", file(json!({ "$schema": schema, "supportForTags": { "@nope": true } }))),
                ("tsdoc.json", file(json!({ "$schema": schema, "extends": ["./base.json"] }))),
            ],
            Id::ConfigFileUndefinedTag,
        ),
        (
            vec![
                ("base.json", file(json!({ "$schema": schema, "extends": ["./tsdoc.json"] }))),
                ("tsdoc.json", file(json!({ "$schema": schema, "extends": ["./base.json"] }))),
            ],
            Id::ConfigFileCyclicExtends,
        ),
    ];
    let folder = tc.draw(gs::sampled_from(vec!["", "a/", "a/b/"]));
    for (files, id) in cases {
        let dir = tempfile::tempdir().unwrap();
        for (name, text) in &files {
            write(dir.path(), &format!("{folder}{name}"), text);
        }
        let path = dir.path().join(format!("{folder}tsdoc.json"));
        let error = config_file::load(&path).unwrap_err();
        assert_eq!(error.message_id, id, "{error}");
        assert!(error.to_string().contains(&format!(".json: {}: ", id.as_str())), "{error}");
    }
}

/// The tsdoc.json found for a source file is the one beside the nearest tsconfig.json or
/// package.json above it, if that folder has one, and the files looked at to find it come back.
#[hegel::test(test_cases = 30)]
fn the_nearest_project_folder_has_the_tsdoc_json(tc: TestCase) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let marker = tc.draw(gs::sampled_from(vec!["tsconfig.json", "package.json"]));
    let project = tc.draw(gs::sampled_from(vec!["", "p/", "p/q/"]));
    write(root, &format!("{project}{marker}"), "{}");
    let has_config = tc.draw(gs::booleans());
    if has_config {
        write(root, &format!("{project}tsdoc.json"), "{}");
    }
    let below = tc.draw(gs::sampled_from(vec!["", "src/", "src/deep/"]));
    let folder = root.join(format!("{project}{below}"));
    fs::create_dir_all(&folder).unwrap();
    let (found, looked) = config_file::find(&folder);
    let config = root.join(format!("{project}tsdoc.json"));
    assert_eq!(
        found.map(|found| fs::canonicalize(found).unwrap()),
        has_config.then(|| fs::canonicalize(&config).unwrap())
    );
    assert_eq!(looked.last().map(|last| last.file_name().unwrap().to_owned()), Some("tsdoc.json".into()));
}
