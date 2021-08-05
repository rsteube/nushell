use std::borrow::Cow;

use nu_parser::NewlineMode;
use nu_source::Tag;

use crate::carapace::CarapaceCompleter;
use crate::command::CommandCompleter;
use crate::engine;
use crate::matchers;
use crate::matchers::Matcher;
use crate::path::PathSuggestion;
use crate::variable::VariableCompleter;
use crate::{Completer, CompletionContext, Suggestion};

pub struct NuCompleter {}

impl NuCompleter {}

impl NuCompleter {
    pub fn complete<Context: CompletionContext>(
        &self,
        line: &str,
        pos: usize,
        context: &Context,
    ) -> (usize, Vec<Suggestion>) {
        use engine::LocationType;

        let tokens = nu_parser::lex(line, 0, NewlineMode::Normal).0;

        let locations = Some(nu_parser::parse_block(tokens).0)
            .map(|block| nu_parser::classify_block(&block, context.scope()))
            .map(|(block, _)| engine::completion_location(line, &block, pos))
            .unwrap_or_default();

        let matcher = nu_data::config::config(Tag::unknown())
            .ok()
            .and_then(|cfg| cfg.get("line_editor").cloned())
            .and_then(|le| {
                le.row_entries()
                    .find(|(idx, _value)| idx.as_str() == "completion_match_method")
                    .and_then(|(_idx, value)| value.as_string().ok())
            })
            .unwrap_or_else(String::new);

        let matcher = matcher.as_str();
        let matcher: &dyn Matcher = match matcher {
            "case-insensitive" => &matchers::case_insensitive::Matcher,
            "case-sensitive" => &matchers::case_sensitive::Matcher,
            #[cfg(target_os = "windows")]
            _ => &matchers::case_insensitive::Matcher,
            #[cfg(not(target_os = "windows"))]
            _ => &matchers::case_sensitive::Matcher,
        };

        let cursor_pos = pos;
        if locations.is_empty() {
            (pos, Vec::new())
        } else {
            let mut pos = locations[0].span.start();
            let mut words = Vec::new();

            
            for token in nu_parser::lex(line, 0, NewlineMode::Normal).0 {
                if token.span.start() <= cursor_pos && token.span.end() >= cursor_pos {
                    words.push(&line[token.span.start()..token.span.end()]);
                    break;
                } else if token.span.end() < cursor_pos {
                    words.push(token.span.slice(line));
                }
            }

            for location in &locations {
                if location.span.start() <= cursor_pos && location.span.end() >= cursor_pos {
                    pos = location.span.start();
                    // break; TODO arrr???
                }
            }

            let suggestions = locations
                .into_iter()
                .flat_map(|location| {
                    let partial = location.span.slice(line).to_string();
                    match location.item {
                        LocationType::Command => {
                            let command_completer = CommandCompleter;
                            command_completer.complete(context, &partial, matcher.to_owned())
                        }

                        LocationType::Variable => {
                            let variable_completer = VariableCompleter;
                            variable_completer.complete(context, &partial, matcher.to_owned())
                        }

                        _ => {
                            // TODO filter locations to just complete the relevant one
                            if location.span.start() <= cursor_pos
                                && location.span.end() >= cursor_pos
                            {
                                if partial.len() == 0 {
                                    words.push("") // TODO current word being completed is empty
                                }

                                let carapace_completer = CarapaceCompleter {
                                    words: words.clone(),
                                };
                                carapace_completer.complete(context, &partial, matcher.to_owned())
                            // TODO fallback to default flag/argument completion (readd code) if no
                            // know command - or rather configure which commands to complete with
                            // carapace
                            } else {
                                Vec::new()
                            }
                        }
                    }
                })
                .collect();

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

fn requote(orig_value: String, previously_quoted: bool) -> String {
    let value: Cow<str> = {
        #[cfg(feature = "rustyline-support")]
        {
            rustyline::completion::unescape(&orig_value, Some('\\'))
        }
        #[cfg(not(feature = "rustyline-support"))]
        {
            orig_value.into()
        }
    };

    let mut quotes = vec!['"', '\''];
    let mut should_quote = false;
    for c in value.chars() {
        if c.is_whitespace() || c == '#' {
            should_quote = true;
        } else if let Some(index) = quotes.iter().position(|q| *q == c) {
            should_quote = true;
            quotes.swap_remove(index);
        }
    }

    if should_quote {
        if quotes.is_empty() {
            // TODO we don't really have an escape character, so there isn't a great option right
            //      now. One possibility is `{{(char backtick)}}`
            value.to_string()
        } else {
            let quote = quotes[0];
            if previously_quoted {
                format!("{}{}", quote, value)
            } else {
                format!("{}{}{}", quote, value, quote)
            }
        }
    } else {
        value.to_string()
    }
}
