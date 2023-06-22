#![allow(clippy::into_iter_on_ref)]
use std::{collections::HashMap, env, fs::File, io::BufWriter, process::Command};

use ropey::Rope;

#[macro_export]
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Location {
    file_path: String,
    line: usize,
    column: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct MethodDefinitionLocationAndName {
    location: Location,
    method_name: String,
}

fn parse_target_method_definition(match_text: &str) -> MethodDefinitionLocationAndName {
    let captures = regex!(r#"^([^:]+):(\d+):(\d+):.* fn ([a-z_]+)"#)
        .captures(match_text)
        .unwrap_or_else(|| {
            panic!("target method definition regex didn't match line: '{match_text}'")
        });
    let file_path = captures[1].to_owned();
    let line: usize = captures[2].parse::<usize>().unwrap() - 1;
    let column: usize = captures[3].parse::<usize>().unwrap() - 1;
    let method_name = captures[4].to_owned();
    MethodDefinitionLocationAndName {
        location: Location {
            line,
            column,
            file_path,
        },
        method_name,
    }
}

fn parse_target_method_definitions(matches_text: &str) -> Vec<MethodDefinitionLocationAndName> {
    matches_text
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(parse_target_method_definition)
        .collect()
}

fn get_target_method_definitions() -> Vec<MethodDefinitionLocationAndName> {
    let output = Command::new("/Users/jrosse/prj/tree-sitter-grep/target/release/tree-sitter-grep")
        .args([
            "-q",
            r#"(function_item
               (visibility_modifier)
               name: (identifier) @function_name
                 (#match? @function_name "^create_(.+)")
                 (#not-match? @function_name "_raw$")
                 (#not-match? @function_name "_worker$")
                 (#not-match? @function_name "^create_base_")
               return_type: (type_identifier)
             )"#,
            "-l",
            "rust",
            "--vimgrep",
            "./src/compiler/factory/node_factory",
        ])
        .output()
        .unwrap();
    parse_target_method_definitions(std::str::from_utf8(&output.stdout).unwrap())
}

fn replace_range_in_file(
    file_path: &str,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    replacement: &str,
) {
    let mut file_text = Rope::from_reader(File::open(file_path).unwrap()).unwrap();

    let start_index = file_text.line_to_char(start_line) + start_column;
    let end_index = file_text.line_to_char(end_line) + end_column;
    file_text.remove(start_index..end_index);
    file_text.insert(start_index, replacement);

    file_text
        .write_to(BufWriter::new(File::create(file_path).unwrap()))
        .unwrap();
}

fn remove_range_in_file(
    file_path: &str,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
) {
    let mut file_text = Rope::from_reader(File::open(file_path).unwrap()).unwrap();

    let start_index = file_text.line_to_char(start_line) + start_column;
    let end_index = file_text.line_to_char(end_line) + end_column;
    file_text.remove(start_index..end_index);

    file_text
        .write_to(BufWriter::new(File::create(file_path).unwrap()))
        .unwrap();
}

fn insert_before_line_in_file(file_path: &str, start_line: usize, addition: &str) {
    let mut file_text = Rope::from_reader(File::open(file_path).unwrap()).unwrap();

    let start_index = file_text.line_to_char(start_line);
    file_text.insert(start_index, addition);

    file_text
        .write_to(BufWriter::new(File::create(file_path).unwrap()))
        .unwrap();
}

fn rename_target_method_definition_and_add_macro_attribute(
    target_method_definition: &MethodDefinitionLocationAndName,
) {
    replace_range_in_file(
        &target_method_definition.location.file_path,
        target_method_definition.location.line,
        target_method_definition.location.column,
        target_method_definition.location.line,
        target_method_definition.location.column + target_method_definition.method_name.len(),
        &format!("{}_raw", target_method_definition.method_name),
    );
    insert_before_line_in_file(
        &target_method_definition.location.file_path,
        target_method_definition.location.line,
        "#[generate_node_factory_method_wrapper]\n",
    );
}

fn rename_target_method_definitions_and_add_macro_attribute(
    target_method_definitions: &[MethodDefinitionLocationAndName],
) {
    for target_method_definition in target_method_definitions {
        rename_target_method_definition_and_add_macro_attribute(target_method_definition);
    }
}

fn rename_target_method_invocation(target_method_invocation: &MethodCallLocation) {
    replace_range_in_file(
        &target_method_invocation.location.file_path,
        target_method_invocation.location.line,
        target_method_invocation.location.column,
        target_method_invocation.location.line,
        target_method_invocation.location.column
            + target_method_invocation.method_definition.method_name.len(),
        &format!(
            "{}_raw",
            target_method_invocation.method_definition.method_name
        ),
    );
}

fn rename_target_method_invocations(target_method_invocations: &[MethodCallLocation]) {
    for target_method_invocation in target_method_invocations {
        rename_target_method_invocation(target_method_invocation);
    }
}

fn rename_target_method_invocation_with_wrap(
    target_method_invocation_with_wrap: &MethodCallLocation,
) {
    replace_range_in_file(
        &target_method_invocation_with_wrap.location.file_path,
        target_method_invocation_with_wrap.location.line,
        target_method_invocation_with_wrap.location.column,
        target_method_invocation_with_wrap.location.line,
        target_method_invocation_with_wrap.location.column
            + target_method_invocation_with_wrap
                .method_definition
                .method_name
                .len()
            + "_raw".len(),
        &target_method_invocation_with_wrap
            .method_definition
            .method_name,
    );
}

fn rename_target_method_invocations_with_wrap(
    target_method_invocations_with_wrap: &[MethodCallLocation],
) {
    for target_method_invocation_with_wrap in target_method_invocations_with_wrap {
        rename_target_method_invocation_with_wrap(target_method_invocation_with_wrap);
    }
}

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
struct MethodCallLocation {
    location: Location,
    method_definition: MethodDefinitionLocationAndName,
}

fn get_target_method_invocation_query(
    target_method_definitions_by_name: &HashMap<String, MethodDefinitionLocationAndName>,
    append_raw: bool,
) -> String {
    format!(
        r#"(call_expression
             function: (field_expression
               field: (field_identifier) @method_name (#match? @method_name "{}")
             )
           )"#,
        target_method_definitions_by_name
            .keys()
            .map(|method_name| format!(
                "(?:^{method_name}{}$)",
                append_raw.then_some("_raw").unwrap_or_default()
            ))
            .collect::<Vec<_>>()
            .join("|")
    )
}

fn get_target_method_invocations(
    target_method_definitions_by_name: &HashMap<String, MethodDefinitionLocationAndName>,
) -> Vec<MethodCallLocation> {
    let output = Command::new("/Users/jrosse/prj/tree-sitter-grep/target/release/tree-sitter-grep")
        .args([
            "-q",
            &get_target_method_invocation_query(target_method_definitions_by_name, false),
            "-l",
            "rust",
            "--vimgrep",
            "./src/compiler",
        ])
        .output()
        .unwrap();
    parse_target_method_invocations(
        std::str::from_utf8(&output.stdout).unwrap(),
        target_method_definitions_by_name,
        false,
    )
}

fn get_wrap_query(
    target_method_definitions_by_name: &HashMap<String, MethodDefinitionLocationAndName>,
) -> String {
    format!(
        r#"(call_expression
             function: (field_expression
               value: {}
               field: (field_identifier) @wrap (#eq? @wrap "wrap")
             )
        )"#,
        get_target_method_invocation_query(target_method_definitions_by_name, true)
    )
}

fn get_target_method_invocations_with_wrap(
    target_method_definitions_by_name: &HashMap<String, MethodDefinitionLocationAndName>,
) -> Vec<MethodCallLocation> {
    let output = Command::new("/Users/jrosse/prj/tree-sitter-grep/target/release/tree-sitter-grep")
        .args([
            "-q",
            &get_wrap_query(target_method_definitions_by_name),
            "-l",
            "rust",
            "--vimgrep",
            "./src/compiler",
        ])
        .output()
        .unwrap();
    parse_target_method_invocations(
        std::str::from_utf8(&output.stdout).unwrap(),
        target_method_definitions_by_name,
        true,
    )
}

fn get_wrap_invocations_to_remove(
    target_method_definitions_by_name: &HashMap<String, MethodDefinitionLocationAndName>,
) -> Vec<Location> {
    let output = Command::new("/Users/jrosse/prj/tree-sitter-grep/target/release/tree-sitter-grep")
        .args([
            "-q",
            &get_wrap_query(target_method_definitions_by_name),
            "-l",
            "rust",
            "--vimgrep",
            "--capture",
            "wrap",
            "./src/compiler",
        ])
        .output()
        .unwrap();
    parse_wrap_invocations(std::str::from_utf8(&output.stdout).unwrap())
}

fn parse_target_method_invocation(
    match_text: &str,
    target_method_definitions_by_name: &HashMap<String, MethodDefinitionLocationAndName>,
    should_strip_raw_suffix: bool,
) -> MethodCallLocation {
    let captures = regex!(r#"^([^:]+):(\d+):(\d+):(.+)"#)
        .captures(match_text)
        .unwrap_or_else(|| {
            panic!("target method invocation regex didn't match line: '{match_text}'")
        });
    let file_path = captures[1].to_owned();
    let line: usize = captures[2].parse::<usize>().unwrap() - 1;
    let column: usize = captures[3].parse::<usize>().unwrap() - 1;
    let line_text = &captures[4];
    let method_name_match = regex!(r#"^[a-z_]+"#)
        .captures(&line_text[column..])
        .unwrap_or_else(|| {
            panic!("couldn't find invoked method name at column position in line: '{match_text}'")
        });
    let mut method_name = &method_name_match[0];
    if should_strip_raw_suffix {
        assert!(method_name.ends_with("_raw"));
        method_name = &method_name[..method_name.len() - 4];
    }
    let target_method_definition = target_method_definitions_by_name
        .get(method_name)
        .cloned()
        // .unwrap_or_else(|| panic!("found method name wasn't known: '{method_name}'"));
        .unwrap_or_else(|| panic!("method_name: {method_name:?}, line_text: {line_text:?}, match_text: {match_text:?}"));
    MethodCallLocation {
        location: Location {
            line,
            column,
            file_path,
        },
        method_definition: target_method_definition,
    }
}

fn parse_target_method_invocations(
    matches_text: &str,
    target_method_definitions_by_name: &HashMap<String, MethodDefinitionLocationAndName>,
    should_strip_raw_suffix: bool,
) -> Vec<MethodCallLocation> {
    matches_text
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            parse_target_method_invocation(
                line,
                target_method_definitions_by_name,
                should_strip_raw_suffix,
            )
        })
        .collect()
}

fn parse_wrap_invocation(match_text: &str) -> Location {
    let captures = regex!(r#"^([^:]+):(\d+):(\d+):"#)
        .captures(match_text)
        .unwrap_or_else(|| panic!("wrap invocation regex didn't match line: '{match_text}'"));
    let file_path = captures[1].to_owned();
    let line: usize = captures[2].parse::<usize>().unwrap() - 1;
    let column: usize = captures[3].parse::<usize>().unwrap() - 1;
    Location {
        line,
        column,
        file_path,
    }
}

fn parse_wrap_invocations(matches_text: &str) -> Vec<Location> {
    matches_text
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(parse_wrap_invocation)
        .collect()
}

fn remove_wrap_invocation(wrap_invocation: &Location) {
    remove_range_in_file(
        &wrap_invocation.file_path,
        wrap_invocation.line,
        wrap_invocation.column - 1,
        wrap_invocation.line,
        wrap_invocation.column + 6,
    );
}

fn remove_wrap_invocations(wrap_invocations: &[Location]) {
    for wrap_invocation in wrap_invocations {
        remove_wrap_invocation(wrap_invocation);
    }
}

fn main() {
    env::set_current_dir("/Users/jrosse/prj/tsc-rust/typescript_rust").unwrap();

    let mut target_method_definitions = get_target_method_definitions();
    target_method_definitions.sort();
    target_method_definitions.reverse();
    let target_method_definitions_by_name: HashMap<_, _> = target_method_definitions
        .iter()
        .map(|target_method_definition| {
            (
                target_method_definition.method_name.clone(),
                target_method_definition.clone(),
            )
        })
        .collect();
    rename_target_method_definitions_and_add_macro_attribute(&target_method_definitions);

    let mut target_method_invocations =
        get_target_method_invocations(&target_method_definitions_by_name);
    target_method_invocations.sort();
    target_method_invocations.reverse();
    rename_target_method_invocations(&target_method_invocations);

    let mut target_method_invocations_with_wrap =
        get_target_method_invocations_with_wrap(&target_method_definitions_by_name);
    target_method_invocations_with_wrap.sort();
    target_method_invocations_with_wrap.reverse();
    let mut wrap_invocations_to_remove =
        get_wrap_invocations_to_remove(&target_method_definitions_by_name);
    wrap_invocations_to_remove.sort();
    wrap_invocations_to_remove.reverse();
    assert_eq!(
        target_method_invocations_with_wrap.len(),
        wrap_invocations_to_remove.len()
    );
    remove_wrap_invocations(&wrap_invocations_to_remove);
    rename_target_method_invocations_with_wrap(&target_method_invocations_with_wrap);
}
