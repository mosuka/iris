//! Interactive Read-Eval-Print Loop (REPL) for the laurus search engine.
//!
//! Provides a terminal-based interactive session where users can search the
//! index, add/get/delete fields and documents, commit changes, and view index
//! statistics without restarting the CLI. Command history is supported via
//! `rustyline`.
//!
//! Commands follow the same `<operation> <resource>` ordering as the CLI
//! (e.g. `add doc`, `delete field`, `get index`).

use std::path::Path;

use anyhow::{Context, Result};
use laurus::lexical::query::parser::QueryParser;
use laurus::{Document, Engine, FieldOption, LexicalSearchRequest, Schema, SearchRequestBuilder};
use rustyline::DefaultEditor;

use crate::context;
use crate::output::{self, OutputFormat};

/// Run the interactive REPL session.
///
/// Opens the index and enters a read-eval-print loop that accepts commands
/// from the user via a `rustyline` prompt. Supported commands mirror the CLI
/// structure: `search`, `add`, `get`, `delete`, `commit`, `help`, and `quit`.
///
/// # Arguments
///
/// * `index_dir` - Path to the index directory holding the index.
/// * `format` - The desired output format (table or JSON) for results.
///
/// # Returns
///
/// Returns `Ok(())` when the user exits the REPL via `quit`, `exit`, Ctrl-C,
/// or Ctrl-D.
///
/// # Errors
///
/// Returns an error if:
/// - The index cannot be opened.
/// - The schema file cannot be read or parsed.
/// - The readline editor fails to initialise.
pub async fn run(index_dir: &Path, format: OutputFormat) -> Result<()> {
    let engine = context::open_index(index_dir).await?;

    // Read schema for default fields.
    let schema_toml = std::fs::read_to_string(index_dir.join("schema.toml"))
        .context("Failed to read schema file")?;
    let schema: Schema = toml::from_str(&schema_toml).context("Failed to parse schema TOML")?;

    let mut rl = DefaultEditor::new()?;

    println!("Laurus REPL (type 'help' for commands, 'quit' to exit)");

    loop {
        let line = match rl.readline("laurus> ") {
            Ok(line) => line,
            Err(
                rustyline::error::ReadlineError::Interrupted | rustyline::error::ReadlineError::Eof,
            ) => {
                break;
            }
            Err(err) => {
                eprintln!("Error: {err}");
                break;
            }
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let _ = rl.add_history_entry(line);

        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        let result = match parts[0] {
            "help" => {
                print_help();
                Ok(())
            }
            "quit" | "exit" => break,
            "search" => {
                if parts.len() < 2 {
                    eprintln!("Usage: search <query>");
                    continue;
                }
                let query_str = parts[1..].join(" ");
                handle_search(&engine, &schema, &query_str, format).await
            }
            "add" => {
                if parts.len() < 2 {
                    eprintln!("Usage: add <field|doc> ...");
                    continue;
                }
                handle_add(&engine, index_dir, parts[1], parts.get(2).copied()).await
            }
            "get" => {
                if parts.len() < 2 {
                    eprintln!("Usage: get <stats|schema|doc> ...");
                    continue;
                }
                handle_get(&engine, index_dir, parts[1], parts.get(2).copied(), format).await
            }
            "delete" => {
                if parts.len() < 2 {
                    eprintln!("Usage: delete <field|doc> ...");
                    continue;
                }
                handle_delete(&engine, index_dir, parts[1], parts.get(2).copied()).await
            }
            "commit" => {
                engine.commit().await?;
                println!("Changes committed.");
                Ok(())
            }
            _ => {
                eprintln!(
                    "Unknown command: '{}'. Type 'help' for available commands.",
                    parts[0]
                );
                Ok(())
            }
        };

        if let Err(e) = result {
            eprintln!("Error: {e:#}");
        }
    }

    println!("Goodbye.");
    Ok(())
}

fn print_help() {
    println!(
        "\
Available commands:
  search <query>               Search the index
  add field <name> <json>      Add a field to the schema
  add doc <id> <json>          Add a document
  get stats                    Show index statistics
  get schema                   Show current schema
  get doc <id>                 Get a document by ID
  delete field <name>          Remove a field from the schema
  delete doc <id>              Delete a document by ID
  commit                       Commit pending changes
  help                         Show this help
  quit                         Exit the REPL"
    );
}

async fn handle_search(
    engine: &Engine,
    schema: &Schema,
    query_str: &str,
    format: OutputFormat,
) -> Result<()> {
    let mut parser =
        QueryParser::with_standard_analyzer().context("Failed to create query parser")?;
    if !schema.default_fields.is_empty() {
        parser = parser.with_default_fields(schema.default_fields.clone());
    }

    let query = parser.parse(query_str)?;
    let request = SearchRequestBuilder::new()
        .lexical_search_request(LexicalSearchRequest::new(query))
        .limit(10)
        .build();

    let results = engine.search(request).await?;
    output::print_search_results(&results, format);
    Ok(())
}

/// Handle `add field ...` and `add doc ...` commands.
async fn handle_add(
    engine: &Engine,
    index_dir: &Path,
    resource: &str,
    rest: Option<&str>,
) -> Result<()> {
    match resource {
        "field" => {
            let rest = rest.context("Usage: add field <name> <json>")?;
            let (name, json_str) = rest
                .split_once(' ')
                .context("Usage: add field <name> <json>")?;
            let field_option: FieldOption =
                serde_json::from_str(json_str).context("Failed to parse field option JSON")?;
            let updated_schema = engine
                .add_field(name, field_option)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            context::save_schema(index_dir, &updated_schema)?;
            println!("Field '{name}' added.");
            Ok(())
        }
        "doc" => {
            let rest = rest.context("Usage: add doc <id> <json>")?;
            let (id, json_str) = rest.split_once(' ').context("Usage: add doc <id> <json>")?;
            let doc: Document =
                serde_json::from_str(json_str).context("Failed to parse document JSON")?;
            engine.add_document(id, doc).await?;
            println!("Document '{id}' added.");
            Ok(())
        }
        _ => {
            eprintln!("Unknown resource: '{resource}'. Use field or doc.");
            Ok(())
        }
    }
}

/// Handle `get index`, `get schema`, and `get doc ...` commands.
async fn handle_get(
    engine: &Engine,
    index_dir: &Path,
    resource: &str,
    rest: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    match resource {
        "stats" => {
            let stats = engine.stats()?;
            output::print_stats(&stats, format);
            Ok(())
        }
        "schema" => {
            let schema = context::read_schema(index_dir)?;
            let json = serde_json::to_string_pretty(&schema)
                .context("Failed to serialize schema to JSON")?;
            println!("{json}");
            Ok(())
        }
        "doc" => {
            let id = rest.context("Usage: get doc <id>")?;
            let documents = engine.get_documents(id).await?;
            output::print_documents(id, &documents, format);
            Ok(())
        }
        _ => {
            eprintln!("Unknown resource: '{resource}'. Use stats, schema, or doc.");
            Ok(())
        }
    }
}

/// Handle `delete field ...` and `delete doc ...` commands.
async fn handle_delete(
    engine: &Engine,
    index_dir: &Path,
    resource: &str,
    rest: Option<&str>,
) -> Result<()> {
    match resource {
        "field" => {
            let name = rest.context("Usage: delete field <name>")?;
            let updated_schema = engine
                .delete_field(name)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            context::save_schema(index_dir, &updated_schema)?;
            println!("Field '{name}' deleted.");
            Ok(())
        }
        "doc" => {
            let id = rest.context("Usage: delete doc <id>")?;
            engine.delete_documents(id).await?;
            println!("Document '{id}' deleted.");
            Ok(())
        }
        _ => {
            eprintln!("Unknown resource: '{resource}'. Use field or doc.");
            Ok(())
        }
    }
}
