//! Building value trees from parsed table entries.
//!
//! The resolver and [`AstNode::to_value`](crate::parser::ast::AstNode::to_value)
//! both turn `key = value` entries into nested [`Value`] tables. This module
//! holds the shared rules so they cannot drift apart:
//!
//! - key segments are used as written (a quoted key like `"a.b"` stays one key),
//! - tables defined in several places are merged (`[a.b]` before `[a]`,
//!   `a.b = 1` next to `[a]`),
//! - defining the same non-table key twice is an error instead of a silent
//!   overwrite,
//! - a dotted key that walks through an array of tables continues in its last
//!   element, as in TOML.

use crate::error::{NomlError, Result};
use crate::parser::ast::{Key, KeySegment};
use crate::value::Value;
use std::collections::BTreeMap;
use std::fmt::Write;

/// One step in a path through a value tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Seg {
    /// A table key
    Key(String),
    /// An array index
    Index(usize),
}

/// Render a path for error messages, e.g. `servers[0].name`.
pub(crate) fn render_path(path: &[Seg]) -> String {
    let mut out = String::new();
    for seg in path {
        match seg {
            Seg::Key(name) => {
                if !out.is_empty() {
                    out.push('.');
                }
                if is_bare_key(name) {
                    out.push_str(name);
                } else {
                    let _ = write!(out, "{name:?}");
                }
            }
            Seg::Index(i) => {
                let _ = write!(out, "[{i}]");
            }
        }
    }
    out
}

/// True if `name` can be written as a bare key.
///
/// Keys starting with a digit or '-' are quoted so they never read back as
/// numbers.
pub(crate) fn is_bare_key(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// The path a key will occupy inside `table` once inserted.
///
/// This is the key's segments, plus an array index wherever the key passes
/// through an existing array of tables.
pub(crate) fn physical_path(table: &BTreeMap<String, Value>, key: &Key) -> Vec<Seg> {
    let segments = &key.segments;
    let mut path = Vec::with_capacity(segments.len() + 1);
    let mut current = Some(table);
    for (i, seg) in segments.iter().enumerate() {
        path.push(Seg::Key(seg.name.clone()));
        if i + 1 == segments.len() {
            break;
        }
        current = match current.and_then(|t| t.get(&seg.name)) {
            Some(Value::Table(t)) => Some(t),
            Some(Value::Array(items)) => match items.last() {
                Some(Value::Table(t)) => {
                    path.push(Seg::Index(items.len() - 1));
                    Some(t)
                }
                _ => None,
            },
            _ => None,
        };
    }
    path
}

/// Insert `value` into `table` under `key`, merging tables.
pub(crate) fn insert(table: &mut BTreeMap<String, Value>, key: &Key, value: Value) -> Result<()> {
    let (last, parents) = match key.segments.split_last() {
        Some(split) => split,
        None => {
            return Err(NomlError::parse(
                "Empty key",
                key.span.start_line,
                key.span.start_column,
            ))
        }
    };

    let mut current = table;
    for (depth, seg) in parents.iter().enumerate() {
        let slot = current
            .entry(seg.name.clone())
            .or_insert_with(Value::empty_table);
        let found = slot.type_name();
        current = match slot {
            Value::Table(t) => t,
            Value::Array(items) => match items.last_mut() {
                Some(Value::Table(t)) => t,
                _ => return Err(conflict(key, &key.segments[..=depth], found)),
            },
            _ => return Err(conflict(key, &key.segments[..=depth], found)),
        };
    }

    match current.get_mut(&last.name) {
        None => {
            current.insert(last.name.clone(), value);
            Ok(())
        }
        Some(Value::Table(existing)) => match value {
            Value::Table(incoming) => merge_tables(existing, incoming, key),
            _ => Err(duplicate(key)),
        },
        Some(_) => Err(duplicate(key)),
    }
}

/// Merge `incoming` into `existing`. Nested tables merge; anything else
/// defined on both sides is a duplicate.
fn merge_tables(
    existing: &mut BTreeMap<String, Value>,
    incoming: BTreeMap<String, Value>,
    key: &Key,
) -> Result<()> {
    for (name, value) in incoming {
        match existing.get_mut(&name) {
            None => {
                existing.insert(name, value);
            }
            Some(Value::Table(inner)) => match value {
                Value::Table(incoming_inner) => {
                    let mut nested = key.clone();
                    nested.segments.push(KeySegment {
                        name,
                        quoted: false,
                        quote_style: None,
                    });
                    merge_tables(inner, incoming_inner, &nested)?;
                }
                _ => return Err(duplicate_child(key, &name)),
            },
            Some(_) => return Err(duplicate_child(key, &name)),
        }
    }
    Ok(())
}

fn key_text(segments: &[KeySegment]) -> String {
    render_path(
        &segments
            .iter()
            .map(|s| Seg::Key(s.name.clone()))
            .collect::<Vec<_>>(),
    )
}

fn duplicate(key: &Key) -> NomlError {
    NomlError::parse(
        format!("Duplicate key '{}'", key_text(&key.segments)),
        key.span.start_line,
        key.span.start_column,
    )
}

fn duplicate_child(key: &Key, child: &str) -> NomlError {
    let mut segments = key.segments.clone();
    segments.push(KeySegment {
        name: child.to_string(),
        quoted: false,
        quote_style: None,
    });
    NomlError::parse(
        format!("Duplicate key '{}'", key_text(&segments)),
        key.span.start_line,
        key.span.start_column,
    )
}

fn conflict(key: &Key, prefix: &[KeySegment], found: &str) -> NomlError {
    NomlError::parse(
        format!(
            "Cannot define '{}': '{}' is already defined as a {found}",
            key_text(&key.segments),
            key_text(prefix)
        ),
        key.span.start_line,
        key.span.start_column,
    )
}

/// Follow `path` from `root`.
pub(crate) fn get<'v>(root: &'v Value, path: &[Seg]) -> Option<&'v Value> {
    let mut current = root;
    for seg in path {
        current = match (current, seg) {
            (Value::Table(t), Seg::Key(k)) => t.get(k)?,
            (Value::Array(a), Seg::Index(i)) => a.get(*i)?,
            _ => return None,
        };
    }
    Some(current)
}

/// Follow `path` from `root`, mutably.
pub(crate) fn get_mut<'v>(root: &'v mut Value, path: &[Seg]) -> Option<&'v mut Value> {
    let mut current = root;
    for seg in path {
        current = match (current, seg) {
            (Value::Table(t), Seg::Key(k)) => t.get_mut(k)?,
            (Value::Array(a), Seg::Index(i)) => a.get_mut(*i)?,
            _ => return None,
        };
    }
    Some(current)
}
