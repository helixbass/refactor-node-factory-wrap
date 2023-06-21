use std::{env, io, process::Command};

#[macro_export]
macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

#[derive(Debug)]
struct Location {
    line: u32,
    column: u32,
    file_path: String,
}

#[derive(Debug)]
struct MethodDefinitionLocationAndName {
    location: Location,
    method_name: String,
}

fn parse_target_method_definition(match_text: &str) -> MethodDefinitionLocationAndName {
    let captures = regex!(r#"([^:]+):(\d+):(\d+):.* fn ([a-z_]+)"#)
        .captures(match_text)
        .expect(&format!(
            "target method definition regex didn't match line: '{match_text}'"
        ));
    let file_path = captures[1].to_owned();
    let line: u32 = captures[2].parse().unwrap();
    let column: u32 = captures[3].parse().unwrap();
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

fn get_target_method_definitions() -> io::Result<Vec<MethodDefinitionLocationAndName>> {
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
        .output()?;
    Ok(parse_target_method_definitions(
        std::str::from_utf8(&output.stdout).unwrap(),
    ))
}

fn main() -> io::Result<()> {
    env::set_current_dir("/Users/jrosse/prj/tsc-rust/typescript_rust")?;

    let target_method_definitions = get_target_method_definitions()?;
    println!("target_method_definitions: {target_method_definitions:#?}");
    Ok(())
}
