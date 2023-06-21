#![allow(clippy::into_iter_on_ref, clippy::expect_fun_call)]
use std::{env, fs::File, io::BufWriter, process::Command};

use ropey::Rope;

#[macro_export]
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

#[derive(Clone, Debug)]
struct Location {
    line: usize,
    column: usize,
    file_path: String,
}

#[derive(Clone, Debug)]
struct MethodDefinitionLocationAndName {
    location: Location,
    method_name: String,
}

fn parse_target_method_definition(match_text: &str) -> MethodDefinitionLocationAndName {
    let captures = regex!(r#"^([^:]+):(\d+):(\d+):.* fn ([a-z_]+)"#)
        .captures(match_text)
        .expect(&format!(
            "target method definition regex didn't match line: '{match_text}'"
        ));
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

fn rename_target_method_definition(target_method_definition: &MethodDefinitionLocationAndName) {
    replace_range_in_file(
        &target_method_definition.location.file_path,
        target_method_definition.location.line,
        target_method_definition.location.column,
        target_method_definition.location.line,
        target_method_definition.location.column + target_method_definition.method_name.len(),
        &format!("{}_raw", target_method_definition.method_name),
    );
}

fn rename_target_method_definitions(target_method_definitions: &[MethodDefinitionLocationAndName]) {
    for target_method_definition in target_method_definitions {
        rename_target_method_definition(target_method_definition);
    }
}

#[derive(Debug)]
struct MethodCallLocation {
    location: Location,
    method_definition: MethodDefinitionLocationAndName,
}

fn get_target_method_invocations(
    target_method_definitions: &[MethodDefinitionLocationAndName],
) -> Vec<MethodCallLocation> {
    target_method_definitions
        .into_iter()
        .flat_map(|target_method_definition| {
            let output =
                Command::new("/Users/jrosse/prj/tree-sitter-grep/target/release/tree-sitter-grep")
                    .args([
                        "-q",
                        &format!(
                            r#"(call_expression
                              function: (field_expression
                                field: (field_identifier) @method_name (#eq? @method_name "{}")
                              )
                             )"#,
                            target_method_definition.method_name
                        ),
                        "-l",
                        "rust",
                        "--vimgrep",
                        "./src/compiler",
                    ])
                    .output()
                    .unwrap();
            parse_target_method_invocations(
                std::str::from_utf8(&output.stdout).unwrap(),
                target_method_definition.clone(),
            )
        })
        .collect()
}

fn parse_target_method_invocation(
    match_text: &str,
    target_method_definition: MethodDefinitionLocationAndName,
) -> MethodCallLocation {
    let captures = regex!(r#"^([^:]+):(\d+):(\d+):"#)
        .captures(match_text)
        .expect(&format!(
            "target method invocation regex didn't match line: '{match_text}'"
        ));
    let file_path = captures[1].to_owned();
    let line: usize = captures[2].parse::<usize>().unwrap() - 1;
    let column: usize = captures[3].parse::<usize>().unwrap() - 1;
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
    target_method_definition: MethodDefinitionLocationAndName,
) -> Vec<MethodCallLocation> {
    matches_text
        .split('\n')
        .filter(|line| !line.is_empty())
        .map(|line| parse_target_method_invocation(line, target_method_definition.clone()))
        .collect()
}

fn main() {
    env::set_current_dir("/Users/jrosse/prj/tsc-rust/typescript_rust").unwrap();

    // let target_method_definitions = get_target_method_definitions();
    let target_method_definitions = get_target_method_definitions()
        .into_iter()
        .take(4)
        .collect::<Vec<_>>();
    let target_method_invocations = get_target_method_invocations(&target_method_definitions);
    println!(
        "target_method_invocations len: {}",
        target_method_invocations.len()
    );
    rename_target_method_definitions(&target_method_definitions);
    // println!("target_method_definitions: {target_method_definitions:#?}");
}
