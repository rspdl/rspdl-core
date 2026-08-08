use std::env;
use std::fs;
use std::process::ExitCode;

use rspdl_compiler::{
    CheckOptions, KoSource, PolicyStatus, WorkspaceCheckReport, check_ko, check_ko_files,
    compile_ko, compile_ko_files,
};
use rspdl_ko::{Diagnostic, ParseOutput, format_document, parse, render_diagnostic};
use serde::Serialize;

#[derive(Serialize)]
struct FileParseOutput {
    path: String,
    output: ParseOutput,
}

#[derive(Serialize)]
struct FileDiagnostics {
    path: String,
    diagnostics: Vec<Diagnostic>,
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            usage();
            ExitCode::from(1)
        }
    }
}

fn run(arguments: Vec<String>) -> Result<ExitCode, String> {
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err("명령이 필요합니다.".into());
    };
    match command {
        "parse" => parse_command(&arguments[1..]),
        "compile" => compile_command(&arguments[1..]),
        "check" => check_command(&arguments[1..]),
        "format" => format_command(&arguments[1..]),
        _ => Err(format!("알 수 없는 명령 `{command}`입니다.")),
    }
}

fn parse_command(arguments: &[String]) -> Result<ExitCode, String> {
    let (paths, json) = source_arguments(arguments, true)?;
    let sources = read_sources(paths)?;
    if sources.len() == 1 {
        let output = parse(&sources[0].text);
        let has_errors = output.diagnostics.iter().any(Diagnostic::is_error);
        if json {
            print_json(&output)?;
        } else {
            print_diagnostics(&output.diagnostics);
        }
        return Ok(if has_errors {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        });
    }
    let outputs = sources
        .into_iter()
        .map(|source| FileParseOutput {
            path: source.path,
            output: parse(&source.text),
        })
        .collect::<Vec<_>>();
    let has_errors = outputs
        .iter()
        .any(|file| file.output.diagnostics.iter().any(Diagnostic::is_error));
    if json {
        print_json(&outputs)?;
    } else {
        for output in &outputs {
            print_file_diagnostics(&output.path, &output.output.diagnostics);
        }
    }
    Ok(if has_errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn compile_command(arguments: &[String]) -> Result<ExitCode, String> {
    let (paths, json) = source_arguments(arguments, true)?;
    let sources = read_sources(paths)?;
    if sources.len() == 1 {
        let compilation = compile_ko(&sources[0].text);
        let has_errors = compilation.diagnostics.iter().any(Diagnostic::is_error);
        if json {
            print_json(&compilation)?;
        } else {
            print_diagnostics(&compilation.diagnostics);
        }
        return Ok(if has_errors {
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        });
    }
    let compilation = compile_ko_files(sources);
    let has_errors = compilation.has_errors();
    if json {
        print_json(&compilation)?;
    } else {
        for file in &compilation.files {
            print_file_diagnostics(&file.path, &file.diagnostics);
        }
    }
    Ok(if has_errors {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn check_command(arguments: &[String]) -> Result<ExitCode, String> {
    let mut paths = Vec::new();
    let mut data_path = None;
    let mut json = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--json" => json = true,
            "--data" => {
                if data_path.is_some() {
                    return Err("`--data`는 한 번만 지정할 수 있습니다.".into());
                }
                index += 1;
                let path = arguments
                    .get(index)
                    .filter(|value| !value.starts_with('-'))
                    .ok_or_else(|| "`--data` 뒤에 JSON 파일 경로가 필요합니다.".to_owned())?;
                data_path = Some(path.clone());
            }
            argument if argument.starts_with('-') => {
                return Err(format!("알 수 없는 인자 `{argument}`입니다."));
            }
            path => paths.push(path.to_owned()),
        }
        index += 1;
    }
    let data_path = data_path.ok_or_else(|| "`--data <input.json>`이 필요합니다.".to_owned())?;
    let sources = read_sources(validate_paths(paths)?)?;
    let data = read(&data_path)?;
    if sources.len() == 1 {
        let report = check_ko(&sources[0].text, &data, CheckOptions::default());
        if json {
            print_json(&report)?;
        } else {
            print_human_report(&report);
        }
        return Ok(if report.has_errors() {
            ExitCode::from(1)
        } else if report.has_findings() {
            ExitCode::from(2)
        } else {
            ExitCode::SUCCESS
        });
    }
    let report = check_ko_files(sources, &data, CheckOptions::default());
    if json {
        print_json(&report)?;
    } else {
        print_workspace_human_report(&report);
    }
    Ok(if report.has_errors() {
        ExitCode::from(1)
    } else if report.has_findings() {
        ExitCode::from(2)
    } else {
        ExitCode::SUCCESS
    })
}

fn format_command(arguments: &[String]) -> Result<ExitCode, String> {
    let (paths, _) = source_arguments(arguments, false)?;
    let sources = read_sources(paths)?;
    let parsed = sources
        .into_iter()
        .map(|source| (source.path, parse(&source.text)))
        .collect::<Vec<_>>();
    if parsed
        .iter()
        .any(|(_, output)| output.diagnostics.iter().any(Diagnostic::is_error))
    {
        let diagnostics = parsed
            .into_iter()
            .map(|(path, output)| FileDiagnostics {
                path,
                diagnostics: output.diagnostics,
            })
            .collect::<Vec<_>>();
        print_json(&diagnostics)?;
        return Ok(ExitCode::from(1));
    }
    let multiple = parsed.len() > 1;
    for (index, (path, output)) in parsed.into_iter().enumerate() {
        let document = output
            .document
            .ok_or_else(|| format!("format할 문서가 없습니다 ({path})."))?;
        if multiple && index > 0 {
            println!();
        }
        print!(
            "{}",
            format_document(&document).map_err(|error| error.to_string())?
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn source_arguments(arguments: &[String], allow_json: bool) -> Result<(Vec<String>, bool), String> {
    let mut paths = Vec::new();
    let mut json = false;
    for argument in arguments {
        if argument == "--json" && allow_json {
            json = true;
        } else if argument.starts_with('-') {
            return Err(format!("알 수 없는 인자 `{argument}`입니다."));
        } else {
            paths.push(argument.clone());
        }
    }
    Ok((validate_paths(paths)?, json))
}

fn validate_paths(mut paths: Vec<String>) -> Result<Vec<String>, String> {
    if paths.is_empty() {
        return Err("RSPDL 파일 경로가 필요합니다.".into());
    }
    paths.sort();
    for pair in paths.windows(2) {
        if pair[0] == pair[1] {
            return Err(format!("RSPDL 파일 `{}`이 중복 지정되었습니다.", pair[0]));
        }
    }
    Ok(paths)
}

fn read_sources(paths: Vec<String>) -> Result<Vec<KoSource>, String> {
    paths
        .into_iter()
        .map(|path| read(&path).map(|text| KoSource::new(path, text)))
        .collect()
}

fn read(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("파일을 읽을 수 없습니다 ({path}): {error}"))
}

fn print_json(value: &impl serde::Serialize) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|error| format!("결과를 JSON으로 만들 수 없습니다: {error}"))?;
    println!("{json}");
    Ok(())
}

fn print_human_report(report: &rspdl_compiler::CheckReport) {
    for diagnostic in &report.compilation.diagnostics {
        eprintln!(
            "{} [{}..{}] {}",
            diagnostic.rule_id,
            diagnostic.span.start,
            diagnostic.span.end,
            render_diagnostic(diagnostic)
        );
    }
    for diagnostic in &report.runtime_diagnostics {
        eprintln!(
            "{} {} {}",
            diagnostic.rule_id, diagnostic.path, diagnostic.message
        );
    }
    for violation in &report.constraint_violations {
        println!(
            "CONSTRAINT {}: {} / {}",
            violation.constraint_id, violation.model_id, violation.record_id
        );
    }
    for result in &report.policy_results {
        let status = match result.status {
            PolicyStatus::Allowed => "ALLOWED",
            PolicyStatus::Denied => "DENIED",
            PolicyStatus::Conflict => "CONFLICT",
            PolicyStatus::Unmatched => "UNMATCHED",
        };
        println!(
            "{status} {} (allow: {}, deny: {})",
            result.request_id,
            ids(&result.allow_policies),
            ids(&result.deny_policies)
        );
    }
    if !report.has_errors() && !report.has_findings() {
        println!("PASS: 제약 및 정책 위반을 찾지 못했습니다.");
    }
}

fn print_diagnostics(diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        eprintln!(
            "{} [{}..{}] {}",
            diagnostic.rule_id,
            diagnostic.span.start,
            diagnostic.span.end,
            render_diagnostic(diagnostic)
        );
    }
}

fn print_file_diagnostics(path: &str, diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        eprintln!(
            "{}: {} [{}..{}] {}",
            path,
            diagnostic.rule_id,
            diagnostic.span.start,
            diagnostic.span.end,
            render_diagnostic(diagnostic)
        );
    }
}

fn print_workspace_human_report(report: &WorkspaceCheckReport) {
    for file in &report.compilation.files {
        for diagnostic in &file.diagnostics {
            eprintln!(
                "{}: {} [{}..{}] {}",
                file.path,
                diagnostic.rule_id,
                diagnostic.span.start,
                diagnostic.span.end,
                render_diagnostic(diagnostic)
            );
        }
    }
    for diagnostic in &report.runtime_diagnostics {
        eprintln!(
            "{} {} {}",
            diagnostic.rule_id, diagnostic.path, diagnostic.message
        );
    }
    for violation in &report.constraint_violations {
        println!(
            "CONSTRAINT {}: {} / {}",
            violation.constraint_id, violation.model_id, violation.record_id
        );
    }
    for result in &report.policy_results {
        let status = match result.status {
            PolicyStatus::Allowed => "ALLOWED",
            PolicyStatus::Denied => "DENIED",
            PolicyStatus::Conflict => "CONFLICT",
            PolicyStatus::Unmatched => "UNMATCHED",
        };
        println!(
            "{status} {} (allow: {}, deny: {})",
            result.request_id,
            ids(&result.allow_policies),
            ids(&result.deny_policies)
        );
    }
    if !report.has_errors() && !report.has_findings() {
        println!("PASS: 제약 및 정책 위반을 찾지 못했습니다.");
    }
}

fn ids(values: &[rspdl_domain::CanonicalId]) -> String {
    if values.is_empty() {
        "-".into()
    } else {
        values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn usage() {
    eprintln!(
        "Usage:\n\
         rspdl parse <file>... [--json]\n\
         rspdl compile <file>... [--json]\n\
         rspdl check <file>... --data <input.json> [--json]\n\
         rspdl format <file>..."
    );
}
