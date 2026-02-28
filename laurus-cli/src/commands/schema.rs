use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use dialoguer::{Confirm, Input, MultiSelect, Select};
use laurus::lexical::core::field::{
    BooleanOption, BytesOption, DateTimeOption, FloatOption, GeoOption, IntegerOption, TextOption,
};
use laurus::vector::core::field::{FlatOption, HnswOption, IvfOption};
use laurus::vector::DistanceMetric;
use laurus::{FieldOption, Schema};

/// Field type names shown in the interactive prompt.
const FIELD_TYPES: &[&str] = &[
    "Text", "Integer", "Float", "Boolean", "DateTime", "Geo", "Bytes", "Hnsw", "Flat", "Ivf",
];

/// Distance metric names shown in the interactive prompt.
const DISTANCE_METRICS: &[&str] = &["Cosine", "Euclidean", "Manhattan", "DotProduct", "Angular"];

/// Run the interactive schema generation.
pub fn run(output: &Path) -> Result<()> {
    println!("\n=== Laurus Schema Generator ===\n");

    let mut fields: HashMap<String, FieldOption> = HashMap::new();
    let mut field_order: Vec<String> = Vec::new();

    loop {
        let name = prompt_field_name(&fields)?;
        let field_option = prompt_field_type_and_options()?;

        println!("\nField \"{}\" ({}) added.\n", name, field_type_label(&field_option));

        field_order.push(name.clone());
        fields.insert(name, field_option);

        if !Confirm::new()
            .with_prompt("Add another field?")
            .default(true)
            .interact()?
        {
            break;
        }
        println!();
    }

    // Collect lexical field names for default field selection.
    let lexical_fields: Vec<&str> = field_order
        .iter()
        .filter(|name| {
            fields
                .get(*name)
                .map(is_lexical_field)
                .unwrap_or(false)
        })
        .map(|s| s.as_str())
        .collect();

    let default_fields = if lexical_fields.is_empty() {
        Vec::new()
    } else {
        prompt_default_fields(&lexical_fields)?
    };

    let schema = Schema {
        fields,
        default_fields,
    };

    // Show preview.
    let toml_str =
        toml::to_string_pretty(&schema).context("Failed to serialize schema to TOML")?;
    println!("\n--- Preview ---");
    println!("{toml_str}");
    println!("---------------\n");

    if !Confirm::new()
        .with_prompt(format!("Write to {}?", output.display()))
        .default(true)
        .interact()?
    {
        println!("Cancelled.");
        return Ok(());
    }

    std::fs::write(output, &toml_str).context("Failed to write schema file")?;
    println!("Schema written to {}.", output.display());

    Ok(())
}

/// Prompt for a unique field name.
fn prompt_field_name(existing: &HashMap<String, FieldOption>) -> Result<String> {
    loop {
        let name: String = Input::new()
            .with_prompt("Field name")
            .interact_text()?;

        if name.is_empty() {
            println!("Field name cannot be empty.");
            continue;
        }

        if existing.contains_key(&name) {
            println!("Field \"{}\" already exists. Please choose a different name.", name);
            continue;
        }

        return Ok(name);
    }
}

/// Prompt for field type selection and then type-specific options.
fn prompt_field_type_and_options() -> Result<FieldOption> {
    let type_index = Select::new()
        .with_prompt("Field type")
        .items(FIELD_TYPES)
        .default(0)
        .interact()?;

    match FIELD_TYPES[type_index] {
        "Text" => prompt_text_option(),
        "Integer" => prompt_indexed_stored_option("Integer"),
        "Float" => prompt_indexed_stored_option("Float"),
        "Boolean" => prompt_indexed_stored_option("Boolean"),
        "DateTime" => prompt_indexed_stored_option("DateTime"),
        "Geo" => prompt_indexed_stored_option("Geo"),
        "Bytes" => prompt_bytes_option(),
        "Hnsw" => prompt_hnsw_option(),
        "Flat" => prompt_flat_option(),
        "Ivf" => prompt_ivf_option(),
        _ => unreachable!(),
    }
}

/// Prompt for TextOption (indexed, stored, term_vectors).
fn prompt_text_option() -> Result<FieldOption> {
    let indexed = Confirm::new()
        .with_prompt("Indexed?")
        .default(true)
        .interact()?;
    let stored = Confirm::new()
        .with_prompt("Stored?")
        .default(true)
        .interact()?;
    let term_vectors = Confirm::new()
        .with_prompt("Term vectors?")
        .default(false)
        .interact()?;

    Ok(FieldOption::Text(TextOption {
        indexed,
        stored,
        term_vectors,
    }))
}

/// Prompt for field types that have indexed + stored options
/// (Integer, Float, Boolean, DateTime, Geo).
fn prompt_indexed_stored_option(type_name: &str) -> Result<FieldOption> {
    let indexed = Confirm::new()
        .with_prompt("Indexed?")
        .default(true)
        .interact()?;
    let stored = Confirm::new()
        .with_prompt("Stored?")
        .default(true)
        .interact()?;

    Ok(match type_name {
        "Integer" => FieldOption::Integer(IntegerOption { indexed, stored }),
        "Float" => FieldOption::Float(FloatOption { indexed, stored }),
        "Boolean" => FieldOption::Boolean(BooleanOption { indexed, stored }),
        "DateTime" => FieldOption::DateTime(DateTimeOption { indexed, stored }),
        "Geo" => FieldOption::Geo(GeoOption { indexed, stored }),
        _ => unreachable!(),
    })
}

/// Prompt for BytesOption (stored only).
fn prompt_bytes_option() -> Result<FieldOption> {
    let stored = Confirm::new()
        .with_prompt("Stored?")
        .default(true)
        .interact()?;
    Ok(FieldOption::Bytes(BytesOption { stored }))
}

/// Prompt for a distance metric selection.
fn prompt_distance_metric() -> Result<DistanceMetric> {
    let idx = Select::new()
        .with_prompt("Distance metric")
        .items(DISTANCE_METRICS)
        .default(0)
        .interact()?;

    Ok(match DISTANCE_METRICS[idx] {
        "Cosine" => DistanceMetric::Cosine,
        "Euclidean" => DistanceMetric::Euclidean,
        "Manhattan" => DistanceMetric::Manhattan,
        "DotProduct" => DistanceMetric::DotProduct,
        "Angular" => DistanceMetric::Angular,
        _ => unreachable!(),
    })
}

/// Prompt for a positive usize value with a default.
fn prompt_usize(prompt: &str, default: usize) -> Result<usize> {
    let val: usize = Input::new()
        .with_prompt(prompt)
        .default(default)
        .interact_text()?;
    Ok(val)
}

/// Prompt for HnswOption.
fn prompt_hnsw_option() -> Result<FieldOption> {
    let dimension = prompt_usize("Dimension", 128)?;
    let distance = prompt_distance_metric()?;
    let m = prompt_usize("M (max connections per node)", 16)?;
    let ef_construction = prompt_usize("ef_construction", 200)?;

    Ok(FieldOption::Hnsw(HnswOption {
        dimension,
        distance,
        m,
        ef_construction,
        base_weight: 1.0,
        quantizer: None,
    }))
}

/// Prompt for FlatOption.
fn prompt_flat_option() -> Result<FieldOption> {
    let dimension = prompt_usize("Dimension", 128)?;
    let distance = prompt_distance_metric()?;

    Ok(FieldOption::Flat(FlatOption {
        dimension,
        distance,
        base_weight: 1.0,
        quantizer: None,
    }))
}

/// Prompt for IvfOption.
fn prompt_ivf_option() -> Result<FieldOption> {
    let dimension = prompt_usize("Dimension", 128)?;
    let distance = prompt_distance_metric()?;
    let n_clusters = prompt_usize("Number of clusters", 100)?;
    let n_probe = prompt_usize("Number of probes", 1)?;

    Ok(FieldOption::Ivf(IvfOption {
        dimension,
        distance,
        n_clusters,
        n_probe,
        base_weight: 1.0,
        quantizer: None,
    }))
}

/// Prompt for default search fields from lexical fields.
fn prompt_default_fields(lexical_fields: &[&str]) -> Result<Vec<String>> {
    if lexical_fields.is_empty() {
        return Ok(Vec::new());
    }

    let selections = MultiSelect::new()
        .with_prompt("Select default search fields")
        .items(lexical_fields)
        .interact()?;

    Ok(selections
        .into_iter()
        .map(|i| lexical_fields[i].to_string())
        .collect())
}

/// Check if a field option is a lexical (non-vector) field type.
fn is_lexical_field(option: &FieldOption) -> bool {
    matches!(
        option,
        FieldOption::Text(_)
            | FieldOption::Integer(_)
            | FieldOption::Float(_)
            | FieldOption::Boolean(_)
            | FieldOption::DateTime(_)
            | FieldOption::Geo(_)
            | FieldOption::Bytes(_)
    )
}

/// Return a human-readable label for a field option variant.
fn field_type_label(option: &FieldOption) -> &'static str {
    match option {
        FieldOption::Text(_) => "Text",
        FieldOption::Integer(_) => "Integer",
        FieldOption::Float(_) => "Float",
        FieldOption::Boolean(_) => "Boolean",
        FieldOption::DateTime(_) => "DateTime",
        FieldOption::Geo(_) => "Geo",
        FieldOption::Bytes(_) => "Bytes",
        FieldOption::Hnsw(_) => "Hnsw",
        FieldOption::Flat(_) => "Flat",
        FieldOption::Ivf(_) => "Ivf",
    }
}
