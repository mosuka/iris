use std::path::Path;

use anyhow::{Context, Result};
use laurus::lexical::query::parser::QueryParser;
use laurus::{LexicalSearchRequest, Schema, SearchRequestBuilder};

use crate::cli::SearchCommand;
use crate::context;
use crate::output::{self, OutputFormat};

/// Execute a search command.
pub async fn run(cmd: SearchCommand, data_dir: &Path, format: OutputFormat) -> Result<()> {
    let engine = context::open_index(data_dir).await?;

    // Read schema to get default fields.
    let schema_toml = std::fs::read_to_string(data_dir.join("schema.toml"))
        .context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_toml).context("Failed to parse schema TOML")?;

    // Build the query parser with default fields from the schema.
    let mut parser =
        QueryParser::with_standard_analyzer().context("Failed to create query parser")?;
    if !schema.default_fields.is_empty() {
        parser = parser.with_default_fields(schema.default_fields);
    }

    // Parse the query string.
    let query = parser.parse(&cmd.query)?;

    // Build and execute the search request.
    let request = SearchRequestBuilder::new()
        .lexical_search_request(LexicalSearchRequest::new(query))
        .limit(cmd.limit)
        .offset(cmd.offset)
        .build();

    let results = engine.search(request).await?;
    output::print_search_results(&results, format);

    Ok(())
}
