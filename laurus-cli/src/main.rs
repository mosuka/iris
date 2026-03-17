//! # laurus-cli
//!
//! Command-line interface binary for the **laurus** search engine.
//!
//! This crate provides the `laurus` CLI executable, which supports creating and
//! managing search indices, adding/retrieving/deleting documents, executing
//! search queries, running an interactive REPL session, and starting a gRPC
//! server.
//!
//! ## Usage
//!
//! ```shell
//! laurus <COMMAND> [OPTIONS]
//! ```
//!
//! Run `laurus --help` for a full list of available commands and options.

mod cli;
mod commands;
mod context;
mod output;

use anyhow::{Context, Result};
use clap::Parser;
use laurus::Document;

use crate::cli::{
    AddResource, Cli, Command, CreateResource, DeleteResource, GetResource, McpCommand,
};
use crate::commands::{mcp, repl, schema, search, serve};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let format = cli.format;
    let data_dir = cli.data_dir;

    match cli.command {
        Command::Create(cmd) => match cmd.resource {
            CreateResource::Index { schema } => {
                context::create_index(&data_dir, &schema).await?;
                println!("Index created at {}.", data_dir.display());
                Ok(())
            }
            CreateResource::Schema { output } => schema::run(&output),
        },
        Command::Get(cmd) => match cmd.resource {
            GetResource::Index => {
                let engine = context::open_index(&data_dir).await?;
                let stats = engine.stats()?;
                output::print_stats(&stats, format);
                Ok(())
            }
            GetResource::Doc { id } => {
                let engine = context::open_index(&data_dir).await?;
                let documents = engine.get_documents(&id).await?;
                output::print_documents(&id, &documents, format);
                Ok(())
            }
        },
        Command::Add(cmd) => match cmd.resource {
            AddResource::Doc { id, data } => {
                let engine = context::open_index(&data_dir).await?;
                let doc: Document =
                    serde_json::from_str(&data).context("Failed to parse document JSON")?;
                engine.add_document(&id, doc).await?;
                println!("Document '{id}' added. Run 'commit' to persist changes.");
                Ok(())
            }
        },
        Command::Delete(cmd) => match cmd.resource {
            DeleteResource::Doc { id } => {
                let engine = context::open_index(&data_dir).await?;
                engine.delete_documents(&id).await?;
                println!("Document '{id}' deleted. Run 'commit' to persist changes.");
                Ok(())
            }
        },
        Command::Commit => {
            let engine = context::open_index(&data_dir).await?;
            engine.commit().await?;
            println!("Changes committed successfully.");
            Ok(())
        }
        Command::Search(cmd) => search::run(cmd, &data_dir, format).await,
        Command::Repl => repl::run(&data_dir, format).await,
        Command::Serve(cmd) => serve::run(cmd, &data_dir).await,
        Command::Mcp(McpCommand { endpoint }) => mcp::run(endpoint.as_deref()).await,
    }
}
