// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025 Tobias Sarnowski <tobias@sarnowski.cloud>

//! LSP server implementation for Lonala.
//!
//! This module implements the Language Server Protocol for Lonala,
//! providing features like semantic tokens for syntax highlighting.

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, SemanticTokens,
    SemanticTokensParams, SemanticTokensResult, SemanticTokensServerCapabilities,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
};
use tower_lsp::{Client, LanguageServer};

use crate::document::Manager;
use crate::semantic_tokens;

/// The Lonala LSP server.
pub struct Lonala {
    client: Client,
    documents: Manager,
}

impl Lonala {
    /// Creates a new LSP server with the given client connection.
    #[inline]
    #[must_use]
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Manager::new(),
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Lonala {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        semantic_tokens::options(),
                    ),
                ),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "lonala-lsp".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Lonala LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        self.documents.open(doc.uri, doc.text, doc.version);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // With TextDocumentSyncKind::FULL, each change contains the complete document text,
        // so there's exactly one change in the vector. We take the first (and only) change.
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.update(
                &params.text_document.uri,
                change.text,
                params.text_document.version,
            );
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.close(&params.text_document.uri);
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;

        let tokens = self
            .documents
            .get(&uri)
            .map(|doc| semantic_tokens::compute(&doc));

        Ok(tokens.map(|data| {
            SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data,
            })
        }))
    }
}
