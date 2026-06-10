use serde::Serialize;

use crate::state::CliError;

/// Types that can render themselves for human-readable terminal output.
pub trait HumanRender {
    /// Produce a human-readable string (no trailing newline required).
    fn render(&self) -> String;
}

/// Render a typed result as either JSON or human-readable output.
pub fn emit<T: Serialize + HumanRender>(value: &T, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{{error: \"{e}\"}}"))
        );
    } else {
        println!("{}", value.render());
    }
}

/// Render an error.  Under `--json`, prints a `{ "kind", "message" }` object.
pub fn emit_error(err: &CliError, json: bool) {
    if json {
        #[derive(Serialize)]
        struct JsonError {
            kind: String,
            message: String,
        }
        let je = JsonError {
            kind: error_kind(err),
            message: err.to_string(),
        };
        println!(
            "{}",
            serde_json::to_string(&je).unwrap_or_else(|e| format!("{{error: \"{e}\"}}"))
        );
    } else {
        eprintln!("Error: {err}");
    }
}

// ---------------------------------------------------------------------------
// HumanRender impl for serde_json::Value — handles all the ad-hoc
// json!({ ... }) structs the commands currently build.
// ---------------------------------------------------------------------------

impl HumanRender for serde_json::Value {
    fn render(&self) -> String {
        match self {
            // Top-level object with a single "message" key: just the message.
            serde_json::Value::Object(map) if map.len() == 1 && map.contains_key("message") => {
                map["message"].to_string().trim_matches('"').to_string()
            }
            other => render_value(other, 0),
        }
    }
}

/// Recursive renderer with indentation for nested objects/arrays.
fn render_value(value: &serde_json::Value, indent: usize) -> String {
    let prefix = " ".repeat(indent);
    match value {
        serde_json::Value::Null => format!("{prefix}null"),
        serde_json::Value::Bool(b) => format!("{prefix}{b}"),
        serde_json::Value::Number(n) => format!("{prefix}{n}"),
        serde_json::Value::String(s) => format!("{prefix}{s}"),
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                return String::new();
            }
            // If all items are strings, join them.
            if arr.iter().all(|v| v.is_string()) {
                return arr
                    .iter()
                    .map(|v| v.as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            // Otherwise render each item.
            let inner: Vec<_> = arr.iter().map(|v| render_value(v, indent)).collect();
            inner.join("\n")
        }
        serde_json::Value::Object(map) => {
            let mut lines = Vec::new();
            for (key, val) in map {
                match val {
                    serde_json::Value::Array(items) if !items.is_empty() => {
                        if let Some(table) = render_table(key, items) {
                            lines.push(table);
                            continue;
                        }
                    }
                    _ => {}
                }
                if val.is_object() {
                    lines.push(render_value(val, indent));
                } else if !val.is_array() {
                    lines.push(format!(
                        "{prefix}{}: {}",
                        key,
                        render_value(val, indent + key.len() + 2).trim_start()
                    ));
                }
            }
            lines.join("\n")
        }
    }
}

/// Attempt to render a homogeneous array of objects as a compact table.
/// Returns `None` if the items aren't all objects with consistent keys.
fn render_table(label: &str, items: &[serde_json::Value]) -> Option<String> {
    let rows: Vec<&serde_json::Map<String, serde_json::Value>> =
        items.iter().filter_map(|v| v.as_object()).collect();
    if rows.len() != items.len() || rows.is_empty() {
        return None;
    }

    // Collect column keys from the first row.
    let keys: Vec<&String> = rows[0].keys().collect();

    // Compute column widths.
    let widths: Vec<usize> = keys
        .iter()
        .map(|k| {
            let header = k.len();
            let data_max = rows
                .iter()
                .map(|r| {
                    r.get(*k)
                        .map(|v| match v {
                            serde_json::Value::String(s) => s.len(),
                            serde_json::Value::Bool(_) => 5,
                            serde_json::Value::Number(n) => n.to_string().len(),
                            _ => 8,
                        })
                        .unwrap_or(0)
                })
                .max()
                .unwrap_or(0);
            header.max(data_max)
        })
        .collect();

    // Pad header columns.
    let header: Vec<String> = keys
        .iter()
        .enumerate()
        .map(|(i, k)| format!("{:>w$}", k.to_uppercase(), w = widths[i]))
        .collect();

    let mut out = Vec::new();
    out.push(format!("{}:", label.to_uppercase()));
    out.push(format!("  {}", header.join("  ")));

    for row in &rows {
        let cells: Vec<String> = keys
            .iter()
            .enumerate()
            .map(|(i, k)| {
                let val = row
                    .get(*k)
                    .map(|v| v.to_string().trim_matches('"').to_string())
                    .unwrap_or_default();
                format!("{:>w$}", val, w = widths[i])
            })
            .collect();
        out.push(format!("  {}", cells.join("  ")));
    }

    Some(out.join("\n"))
}

fn error_kind(err: &CliError) -> String {
    match err {
        CliError::Usage(_) => "usage".into(),
        CliError::NotFound(_) => "notFound".into(),
        CliError::DaemonUnavailable(_) => "unavailable".into(),
        CliError::MfaFailed(_) => "mfaFailed".into(),
        CliError::MfaInputRequired(_) => "mfaInputRequired".into(),
        CliError::NotEnrolled(_) => "notEnrolled".into(),
        CliError::Database(_) => "database".into(),
        CliError::Other(_) => "other".into(),
    }
}
