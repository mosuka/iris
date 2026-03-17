//! CLI subcommand implementations.
//!
//! Each submodule contains the `run` entry-point for its respective CLI
//! subcommand:
//!
//! - [`mcp`] - MCP (Model Context Protocol) server on stdio.
//! - [`repl`] - Interactive Read-Eval-Print Loop session.
//! - [`schema`] - Interactive schema TOML generation wizard.
//! - [`search`] - One-shot search query execution.
//! - [`serve`] - gRPC (and optional HTTP gateway) server.

pub mod mcp;
pub mod repl;
pub mod schema;
pub mod search;
pub mod serve;
