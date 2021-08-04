use super::matchers::Matcher;
use crate::{Completer, CompletionContext, Suggestion};
use std::process::Command;
use serde_json::Value;
use std::str::from_utf8;


pub struct CarapaceCompleter {
    pub(crate) line: String,
    pub(crate) pos: usize,
}

impl<Context> Completer<Context> for CarapaceCompleter
where
    Context: CompletionContext,
{
    fn complete(&self, _ctx: &Context, _partial: &str, _matcher: &dyn Matcher) -> Vec<Suggestion> {
            let substring = self.line[..self.pos].to_owned();
            let words = substring.split_whitespace().collect::<Vec<&str>>();

            if words.len() == 0 {
                Vec::new()
            } else {

            let cmd = words[0];
            let carapace = format!("carapace {}", cmd);
            let prefix = match cmd {
                "example" => "example _carapace",
                _ => &carapace,
            };
            let output = Command::new("sh")
                .arg("-c")
                .arg(format!("{} nushell _ {}''", prefix, &self.line[..self.pos]))
                .output()
                .expect("failed to execute process");
            let output_str = from_utf8(&output.stdout).expect("ignore error");
            let empty = serde_json::from_str("[]").expect("ignore error");
            let v: Value = serde_json::from_str(output_str).unwrap_or(empty);
            let a = v.as_array().expect("ignore error");

            a
                .into_iter()
                .map(|entry| {
                    let mut r = entry["Value"].as_str().expect("ignore error").to_string();
                    if r.contains(" ") {
                        //r = format!("'{}'", r);
                    }
                    Suggestion {
                        replacement: r,
                        display: entry["Display"].as_str().expect("ignore error").to_string(),
                    }
                })
                .collect()
            }
    }
}
