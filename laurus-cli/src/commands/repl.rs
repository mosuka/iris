use std::path::Path;

use anyhow::{Context, Result};
use laurus::lexical::query::parser::QueryParser;
use laurus::{Document, Engine, LexicalSearchRequest, Schema, SearchRequestBuilder};
use rustyline::DefaultEditor;

use crate::context;
use crate::output::{self, OutputFormat};

/// Run the interactive REPL.
pub async fn run(data_dir: &Path, format: OutputFormat) -> Result<()> {
    let engine = context::open_index(data_dir).await?;

    // Read schema for default fields.
    let schema_toml = std::fs::read_to_string(data_dir.join("schema.toml"))
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
                    eprintln!("Usage: search <query> [limit]");
                    continue;
                }
                let query_str = parts[1..].join(" ");
                handle_search(&engine, &schema, &query_str, format).await
            }
            "doc" => {
                if parts.len() < 2 {
                    eprintln!("Usage: doc <add|get|delete> ...");
                    continue;
                }
                handle_doc(&engine, parts[1], parts.get(2).copied(), format).await
            }
            "commit" => {
                engine.commit().await?;
                println!("Changes committed.");
                Ok(())
            }
            "stats" => {
                let stats = engine.stats()?;
                output::print_stats(&stats, format);
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
  search <query> [limit]       Search the index
  doc add <id> <json>          Add a document
  doc get <id>                 Get a document by ID
  doc delete <id>              Delete a document by ID
  commit                       Commit pending changes
  stats                        Show index statistics
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

async fn handle_doc(
    engine: &Engine,
    action: &str,
    rest: Option<&str>,
    format: OutputFormat,
) -> Result<()> {
    match action {
        "add" => {
            let rest = rest.context("Usage: doc add <id> <json>")?;
            let (id, json_str) = rest.split_once(' ').context("Usage: doc add <id> <json>")?;
            let doc: Document =
                serde_json::from_str(json_str).context("Failed to parse document JSON")?;
            engine.add_document(id, doc).await?;
            println!("Document '{id}' added.");
            Ok(())
        }
        "get" => {
            let id = rest.context("Usage: doc get <id>")?;
            let documents = engine.get_documents(id).await?;
            output::print_documents(id, &documents, format);
            Ok(())
        }
        "delete" => {
            let id = rest.context("Usage: doc delete <id>")?;
            engine.delete_documents(id).await?;
            println!("Document '{id}' deleted.");
            Ok(())
        }
        _ => {
            eprintln!("Unknown doc action: '{action}'. Use add, get, or delete.");
            Ok(())
        }
    }
}
