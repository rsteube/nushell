use super::matchers::Matcher;
use crate::{Completer, CompletionContext, Suggestion};
use serde_json::Value;
use std::process::Command;
use std::str::from_utf8;

pub struct CarapaceCompleter<'a> {
    pub(crate) words: Vec<&'a str>,
}

impl<Context> Completer<Context> for CarapaceCompleter<'_>
where
    Context: CompletionContext,
{
    fn complete(&self, _ctx: &Context, _partial: &str, _matcher: &dyn Matcher) -> Vec<Suggestion> {
        if self.words.len() == 0 {
            Vec::new()
        } else {
            let cmd = self.words[0];
            let carapace = format!("carapace {}", cmd);
            let prefix = match cmd {
                "example" => "example _carapace",
                _ => &carapace,
            };
            let output = Command::new(prefix)
                .arg("nushell")
                .arg("_")
                .args(self.words.clone())
                .output()
                .expect("failed to execute process");
            let output_str = from_utf8(&output.stdout).expect("ignore error");
            let empty = serde_json::from_str("[]").expect("ignore error");
            let v: Value = serde_json::from_str(output_str).unwrap_or(empty);
            let a = v.as_array().expect("ignore error");

            a.into_iter()
                .map(|entry| {
                    let r = entry["Value"].as_str().expect("ignore error").to_string();
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
