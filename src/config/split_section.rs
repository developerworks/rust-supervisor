//! Split-section merge and template normalization for supervisor configuration.
//!
//! `rust-config-tree` generates split YAML templates with the section field name
//! at the root, for example `children: []`. Section schemas and runtime merge for
//! body-only split files expect only the section sequence, for example `[]`.
//! Transparent section serialization is handled in Rust; this module adapts YAML
//! arrays back to `confique` nested `{ items: [...] }` form at load time.

use std::path::{Path, PathBuf};

use confique::Config;
use figment::{
    Figment,
    providers::{Format, Json, Serialized, Toml, Yaml},
};
use rust_config_tree::{
    config::{load_config_from_figment, ConfigFormat, ConfigResult, ConfigSchema},
    path::absolutize_lexical,
    tree::{ConfigSource, ConfigTree, ConfigTreeOptions, IncludeOrder},
};
use serde_yaml::{Mapping, Value};

use crate::config::configurable::SupervisorConfig;

/// Maps default split template file names to their root config field names.
const SPLIT_SECTION_FILES: [(&str, &str); 2] = [("groups.yaml", "groups"), ("children.yaml", "children")];

/// Section field names that serialize as transparent arrays in YAML.
const TRANSPARENT_SECTION_FIELDS: [&str; 2] = ["groups", "children"];

/// Builds the Figment graph for supervisor configuration loading.
///
/// Split section files that contain only section-body sequences, such as `[]`, are
/// merged under their section field name. Files that already declare the section
/// root key are merged flat, matching `rust-config-tree` defaults.
pub fn build_supervisor_config_figment(path: impl AsRef<Path>) -> ConfigResult<Figment> {
    let path = path.as_ref();
    load_dotenv_for_path(path)?;

    let tree = load_layer_tree::<SupervisorConfig>(path)?;
    let mut figment = Figment::new();

    for node in tree.nodes().iter().rev() {
        figment = merge_config_file(figment, node.path())?;
    }

    Ok(
        figment.merge(
            rust_config_tree::config::ConfiqueEnvProvider::new::<SupervisorConfig>(),
        ),
    )
}

/// Loads [`SupervisorConfig`] using split-section-aware Figment merging.
pub fn load_supervisor_config(path: impl AsRef<Path>) -> ConfigResult<SupervisorConfig> {
    let figment = build_supervisor_config_figment(path)?;
    load_config_from_figment(&figment)
}

/// Rewrites generated split section templates to section-body-only YAML.
pub fn normalize_generated_split_templates(output_dir: &Path) -> ConfigResult<()> {
    for (file_name, section) in SPLIT_SECTION_FILES {
        let path = output_dir.join(file_name);
        if !path.is_file() {
            continue;
        }

        let content = std::fs::read_to_string(&path)?;
        let normalized = normalize_split_section_template(&content, section);
        if normalized != content {
            std::fs::write(path, normalized)?;
        }
    }

    Ok(())
}

/// Strips redundant section root keys and legacy `items` wrappers from split templates.
pub fn normalize_split_section_template(content: &str, section: &str) -> String {
    let without_section = strip_section_root_key(content, section);
    strip_items_wrapper(&without_section)
}

/// Strips a redundant section root key from one generated split template.
fn strip_section_root_key(content: &str, section: &str) -> String {
    let section_prefix = format!("{section}:");
    let mut lines = content.lines().collect::<Vec<_>>();
    let mut section_line = None;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if *trimmed == section_prefix {
            section_line = Some(index);
        }
        break;
    }

    let Some(section_line) = section_line else {
        return ensure_trailing_newline(content);
    };

    lines.remove(section_line);
    let normalized = lines
        .into_iter()
        .map(|line| {
            if line.starts_with("  ") {
                &line[2..]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    ensure_trailing_newline(&normalized)
}

/// Removes a legacy `items:` wrapper left behind by template generation.
fn strip_items_wrapper(content: &str) -> String {
    let mut lines = content.lines().collect::<Vec<_>>();
    let mut items_line = None;

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "items:" || trimmed.starts_with("items:") {
            items_line = Some(index);
        }
        break;
    }

    let Some(items_line) = items_line else {
        return ensure_trailing_newline(content);
    };

    let inline_value = lines[items_line]
        .trim()
        .strip_prefix("items:")
        .map(str::trim)
        .filter(|value| !value.is_empty());

    lines.remove(items_line);

    if let Some(value) = inline_value {
        lines.insert(items_line, value);
    }

    let normalized = lines
        .into_iter()
        .map(|line| {
            if line.starts_with("  ") {
                &line[2..]
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    ensure_trailing_newline(&normalized)
}

/// Merges one config file into an existing Figment graph.
fn merge_config_file(figment: Figment, path: &Path) -> ConfigResult<Figment> {
    let Some(section) = split_section_for_path(path) else {
        return merge_adapted_file(figment, path);
    };

    if yaml_has_root_key(path, section) {
        return merge_adapted_file(figment, path);
    }

    let body = read_yaml_value(path)?;
    let section_body = adapt_split_section_body(body);
    let mut wrapped = Mapping::new();
    wrapped.insert(Value::String(section.to_string()), section_body);
    Ok(figment.merge(Serialized::defaults(Value::Mapping(wrapped))))
}

/// Returns the split section field name for a known split template file.
fn split_section_for_path(path: &Path) -> Option<&'static str> {
    let file_name = path.file_name()?.to_str()?;
    SPLIT_SECTION_FILES
        .iter()
        .find_map(|(name, section)| (*name == file_name).then_some(*section))
}

/// Returns whether a YAML file declares a top-level mapping key.
fn yaml_has_root_key(path: &Path, key: &str) -> bool {
    if ConfigFormat::from_path(path) != ConfigFormat::Yaml {
        return false;
    }

    Figment::from(Yaml::file(path)).find_value(key).is_ok()
}

/// Reads one YAML document from disk.
fn read_yaml_value(path: &Path) -> ConfigResult<Value> {
    let content = std::fs::read_to_string(path)?;
    serde_yaml::from_str(&content).map_err(|error| {
        figment::Error::from(figment::error::Kind::Message(error.to_string())).into()
    })
}

/// Merges one config file after adapting transparent section arrays for `confique`.
fn merge_adapted_file(figment: Figment, path: &Path) -> ConfigResult<Figment> {
    match ConfigFormat::from_path(path) {
        ConfigFormat::Yaml => {
            let value = read_yaml_value(path)?;
            let adapted = adapt_config_yaml(value, split_section_for_path(path));
            Ok(figment.merge(Serialized::defaults(adapted)))
        }
        ConfigFormat::Toml => Ok(figment.merge(Toml::file(path))),
        ConfigFormat::Json => Ok(figment.merge(Json::file(path))),
    }
}

/// Adapts transparent section arrays into nested `{ items: [...] }` documents.
fn adapt_config_yaml(value: Value, split_file: Option<&str>) -> Value {
    match value {
        Value::Sequence(_) if split_file.is_some() => adapt_split_section_body(value),
        Value::Mapping(map) => {
            let mut adapted = Mapping::new();
            for (key, child) in map {
                let next = if is_transparent_section_key(&key) {
                    adapt_section_value(child)
                } else {
                    adapt_config_yaml(child, None)
                };
                adapted.insert(key, next);
            }
            Value::Mapping(adapted)
        }
        other => other,
    }
}

/// Adapts one split section body into the nested shape expected by `confique`.
fn adapt_split_section_body(value: Value) -> Value {
    match value {
        Value::Sequence(sequence) => wrap_items(sequence),
        Value::Mapping(map) if map.contains_key(Value::String("items".into())) => {
            Value::Mapping(map)
        }
        other => other,
    }
}

/// Adapts one section value that may use transparent array serialization.
fn adapt_section_value(value: Value) -> Value {
    match value {
        Value::Sequence(sequence) => wrap_items(sequence),
        Value::Mapping(map) if map.contains_key(Value::String("items".into())) => {
            Value::Mapping(map)
        }
        other => other,
    }
}

/// Wraps one sequence in the nested `items` field used by section structs.
fn wrap_items(sequence: serde_yaml::Sequence) -> Value {
    let mut map = Mapping::new();
    map.insert(Value::String("items".into()), Value::Sequence(sequence));
    Value::Mapping(map)
}

/// Returns whether a YAML key names a transparent configuration section.
fn is_transparent_section_key(key: &Value) -> bool {
    key.as_str()
        .is_some_and(|name| TRANSPARENT_SECTION_FIELDS.contains(&name))
}

/// Loads one partially parsed configuration layer from disk.
fn load_layer<S>(path: &Path) -> ConfigResult<<S as Config>::Layer>
where
    S: ConfigSchema,
{
    Ok(merge_adapted_file(Figment::new(), path)?.extract()?)
}

/// Loads the recursive config layer tree for supervisor configuration.
fn load_layer_tree<S>(path: &Path) -> ConfigResult<ConfigTree<<S as Config>::Layer>>
where
    S: ConfigSchema,
{
    Ok(ConfigTreeOptions::default()
        .include_order(IncludeOrder::Reverse)
        .load(path, |path| -> ConfigResult<ConfigSource<<S as Config>::Layer>> {
            if split_section_for_path(path).is_some() {
                let layer: <S as Config>::Layer = Figment::new().extract()?;
                return Ok(ConfigSource::new(layer, Vec::new()));
            }

            let layer = load_layer::<S>(path)?;
            let include_paths = S::include_paths(&layer);
            Ok(ConfigSource::new(layer, include_paths))
        })?)
}

/// Loads the nearest ancestor `.env` file for a config path when it exists.
fn load_dotenv_for_path(path: &Path) -> ConfigResult<()> {
    let path = absolutize_lexical(path)?;
    let mut current_dir = path.parent();

    while let Some(dir) = current_dir {
        let dotenv_path = dir.join(".env");
        if dotenv_path.try_exists()? {
            dotenvy::from_path(&dotenv_path)?;
            break;
        }
        current_dir = dir.parent();
    }

    Ok(())
}

/// Ensures generated or rewritten text ends with exactly one newline.
fn ensure_trailing_newline(content: &str) -> String {
    let mut normalized = content.trim_end().to_owned();
    normalized.push('\n');
    normalized
}

/// Resolves the default generated template directory for supervisor configuration.
pub fn default_generated_config_dir() -> PathBuf {
    PathBuf::from("config").join("supervisor_config")
}
