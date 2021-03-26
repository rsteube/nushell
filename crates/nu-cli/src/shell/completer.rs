use crate::completion::path::PathSuggestion;
use crate::completion::{self, Suggestion};
use nu_engine::EvaluationContext;
use nu_parser::ParserScope;
use nu_source::Tag;
use std::process::Command;
//use std::io::{self, Write};
use serde_json::Value;
use std::str::from_utf8;

use std::borrow::Cow;

pub(crate) struct NuCompleter {}

impl NuCompleter {}

impl NuCompleter {
    pub fn complete(
        &self,
        line: &str,
        pos: usize,
        context: &completion::CompletionContext,
    ) -> (usize, Vec<Suggestion>) {
        let nu_context: &EvaluationContext = context.as_ref();

        nu_context.scope.enter_scope();
        let (block, _) = nu_parser::parse(line, 0, &nu_context.scope);
        nu_context.scope.exit_scope();

        let locations = completion::engine::completion_location(line, &block, pos);

        let matcher = nu_data::config::config(Tag::unknown())
            .ok()
            .and_then(|cfg| cfg.get("line_editor").cloned())
            .and_then(|le| {
                le.row_entries()
                    .find(|(idx, _value)| idx.as_str() == "completion_match_method")
                    .and_then(|(_idx, value)| value.as_string().ok())
            })
            .unwrap_or_else(String::new);

        if locations.is_empty() {
            (pos, Vec::new())
        } else {
            let cmd = line.split_whitespace().next().expect("ignore error");
            let carapace = format!("carapace {}", cmd);
            let prefix = match cmd {
                "example" => "example _carapace",
                "gh" => "gh _carapace",
                _ => &carapace,
            };

            let output = Command::new("sh")
                .arg("-c")
                .arg(format!("{} nushell _ {}''", prefix, line))
                .output()
                .expect("failed to execute process");
            let output_str = from_utf8(&output.stdout).expect("ignore error");
            let empty = serde_json::from_str("[]").expect("ignore error");
            let v: Value = serde_json::from_str(output_str).unwrap_or(empty);
            let a = v.as_array().expect("ignore error");

            let suggestions = a
                .into_iter()
                .map(|entry| Suggestion {
                    replacement: entry["Value"].as_str().expect("ignore error").to_string(),
                    display: entry["Display"].as_str().expect("ignore error").to_string(),
                })
                .collect();

            let pos = locations[0].span.start();
            (pos, suggestions)
        }
    }

    fn completeCarapace(
        &self,
        line: &str,
        pos: usize,
        context: &completion::CompletionContext,
    ) -> (usize, Vec<Suggestion>) {
        let nu_context: &EvaluationContext = context.as_ref();

        nu_context.scope.enter_scope();
        let (block, _) = nu_parser::parse(line, 0, &nu_context.scope);
        nu_context.scope.exit_scope();

        let locations = completion::engine::completion_location(line, &block, pos);

        if locations.is_empty() {
            (pos, Vec::new())
        } else {
            let cmd = line.split_whitespace().next().expect("ignore error");
            let carapace = format!("carapace {}", cmd);
            let prefix = match cmd {
                "example" => "example _carapace",
                "gh" => "gh _carapace",
                _ => &carapace,
            };
            let output = Command::new("sh")
                .arg("-c")
                .arg(format!("{} nushell _ {}''", prefix, &line[..pos]))
                .output()
                .expect("failed to execute process");
            let output_str = from_utf8(&output.stdout).expect("ignore error");
            let empty = serde_json::from_str("[]").expect("ignore error");
            let v: Value = serde_json::from_str(output_str).unwrap_or(empty);
            let a = v.as_array().expect("ignore error");

            let suggestions = a
                .into_iter()
                .map(|entry| {
                    let mut r = entry["Value"].as_str().expect("ignore error").to_string();
                    if r.contains(" ") {
                        r = format!("'{}'", r);
                    }
                    Suggestion {
                        replacement: r,
                        display: entry["Display"].as_str().expect("ignore error").to_string(),
                    }
                })
                .collect();

            let pos = locations[0].span.start();
            (pos, suggestions)
        }
    }
}

fn select_directory_suggestions(completed_paths: Vec<PathSuggestion>) -> Vec<PathSuggestion> {
    completed_paths
        .into_iter()
        .filter(|suggestion| {
            suggestion
                .path
                .metadata()
                .map(|md| md.is_dir())
                .unwrap_or(false)
        })
        .collect()
}

fn requote(orig_value: String) -> String {
    let value: Cow<str> = rustyline::completion::unescape(&orig_value, Some('\\'));

    let mut quotes = vec!['"', '\'', '`'];
    let mut should_quote = false;
    for c in value.chars() {
        if c.is_whitespace() {
            should_quote = true;
        } else if let Some(index) = quotes.iter().position(|q| *q == c) {
            should_quote = true;
            quotes.swap_remove(index);
        }
    }

    if should_quote {
        if quotes.is_empty() {
            // TODO we don't really have an escape character, so there isn't a great option right
            //      now. One possibility is `{{$(char backtick)}}`
            value.to_string()
        } else {
            let quote = quotes[0];
            format!("{}{}{}", quote, value, quote)
        }
    } else {
        value.to_string()
    }
}
