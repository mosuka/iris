//! One-shot search query execution.
//!
//! Opens the index, parses the user-supplied query string using the unified
//! query DSL (supporting both lexical and vector clauses), executes the search,
//! and prints the results in the requested output format.

use std::path::Path;

use anyhow::{Context, Result};
use laurus::lexical::query::parser::LexicalQueryParser;
use laurus::vector::query::parser::VectorQueryParser;
use laurus::{Schema, UnifiedQueryParser};

use crate::cli::SearchCommand;
use crate::context;
use crate::output::{self, OutputFormat};

/// Execute a search command against the index.
///
/// Opens the index at `index_dir`, builds a unified query parser (supporting
/// both lexical and vector clauses), parses the query from `cmd`, and prints
/// matching results.
///
/// # Arguments
///
/// * `cmd` - Parsed [`SearchCommand`] containing the query string, limit,
///   and offset.
/// * `index_dir` - Path to the index directory holding the index.
/// * `format` - The desired output format (table or JSON).
///
/// # Returns
///
/// Returns `Ok(())` on success after printing results.
///
/// # Errors
///
/// Returns an error if:
/// - The index cannot be opened.
/// - The schema file cannot be read or parsed.
/// - The query parser cannot be created or the query string is invalid.
/// - The search execution fails.
pub async fn run(cmd: SearchCommand, index_dir: &Path, format: OutputFormat) -> Result<()> {
    let engine = context::open_index(index_dir).await?;

    // Read schema to get default fields.
    let schema_toml = std::fs::read_to_string(index_dir.join("schema.toml"))
        .context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_toml).context("Failed to parse schema TOML")?;

    // Build the lexical parser with default fields from the schema.
    let mut lexical_parser =
        LexicalQueryParser::with_standard_analyzer().context("Failed to create query parser")?;
    if !schema.default_fields.is_empty() {
        lexical_parser = lexical_parser.with_default_fields(schema.default_fields);
    }

    // Build the vector parser using the engine's embedder.
    let embedder = engine.embedder();
    let vector_parser = VectorQueryParser::new(embedder);

    // Collect vector field names from the schema for query routing.
    let vector_fields: std::collections::HashSet<String> = schema
        .fields
        .iter()
        .filter(|(_, opt)| opt.is_vector())
        .map(|(name, _)| name.clone())
        .collect();

    // Create the unified parser that handles both lexical and vector clauses.
    let unified_parser = UnifiedQueryParser::new(lexical_parser, vector_parser, vector_fields);

    // Parse the query string (may embed vector queries asynchronously).
    let mut request = unified_parser
        .parse(&cmd.query)
        .await
        .context("Failed to parse query")?;

    // Apply limit and offset from command-line arguments.
    request.limit = cmd.limit;
    request.offset = cmd.offset;

    let results = engine.search(request).await?;
    output::print_search_results(&results, format);

    Ok(())
}
