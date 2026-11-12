//! The requests that read the resolved program: references, rename, type
//! definition and implementation, signature help, code actions, inlay
//! hints, folding and selection ranges, highlights, workspace symbols and
//! document links. Each is answered from the last check of the package of
//! the document when that check has seen the document as it is now, and
//! waits for the next check otherwise.

use super::*;
use ilang_check::query::{Edit, Query, Reference};
use ilang_syntax::span::Span;

/// The answer of a request that may be refused with a message.
pub(super) type Answer<T> = Result<Result<Option<T>, (ErrorCode, String)>, Later>;

impl Server<'_> {
    /// The document at a URI, in a check that has seen its text as it is.
    pub(super) fn current(&self, uri: &lsp::Uri) -> Result<Option<Place<'_>>, Later> {
        let Some(place) = self.place(uri)? else {
            return Ok(None);
        };
        if !place.change.is_same() {
            return Err(Later::Wait(place.root.clone()));
        }
        Ok(Some(place))
    }

    fn offset(place: &Place, pos: lsp::Position) -> u32 {
        LineIndex::new(place.text).offset(pos)
    }

    /// The LSP location of a span of a file of a check, as the file is now.
    pub(super) fn location(&self, a: &Analysis, source: SourceId, span: Span) -> Result<Option<lsp::Location>, ()> {
        let Some(path) = a.paths.get(&source) else {
            return Ok(None);
        };
        let range = self.range_now(a, source, span).ok_or(())?;
        if self.library_dir().is_some_and(|l| path.starts_with(l)) {
            write_library(a);
        }
        Ok(self.uri_of(path).map(|uri| lsp::Location { uri, range }))
    }

    fn locations(
        &self,
        place: &Place,
        found: impl IntoIterator<Item = query::Location>,
    ) -> Result<Vec<lsp::Location>, Later> {
        let mut out = Vec::new();
        for l in found {
            match self.location(place.a, l.source, l.span) {
                Ok(Some(l)) => out.push(l),
                Ok(None) => {}
                Err(()) => return Err(Later::Wait(place.root.clone())),
            }
        }
        Ok(out)
    }

    fn workspace_edit(&self, place: &Place, edits: Vec<Edit>) -> Result<lsp::WorkspaceEdit, Later> {
        let mut changes: HashMap<lsp::Uri, Vec<lsp::TextEdit>> = HashMap::new();
        for e in edits {
            let Some(path) = place.a.paths.get(&e.source) else {
                continue;
            };
            let Some(uri) = self.uri_of(path) else { continue };
            let range = self
                .range_now(place.a, e.source, e.span)
                .ok_or_else(|| Later::Wait(place.root.clone()))?;
            changes.entry(uri).or_default().push(lsp::TextEdit {
                range,
                new_text: e.text,
            });
        }
        Ok(lsp::WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        })
    }

    pub(super) fn references(&self, p: lsp::ReferenceParams) -> Result<Option<Vec<lsp::Location>>, Later> {
        let pos = p.text_document_position;
        let Some(place) = self.place(&pos.text_document.uri)? else {
            return Ok(None);
        };
        let Some(offset) = Self::old_offset(&place, pos.position) else {
            return place.later();
        };
        let Some(q) = place.a.query() else { return Ok(None) };
        let Some((owner, _, target)) = q.target_at_offset(place.module, offset) else {
            return Ok(None);
        };
        if matches!(target, query::Target::Literal(_)) {
            return Ok(None);
        }
        let refs: Vec<Reference> = q
            .references(&target, owner, &q.own_modules())
            .into_iter()
            .filter(|r| p.context.include_declaration || !r.definition)
            .collect();
        Ok(Some(self.locations(&place, refs.into_iter().map(|r| r.location))?))
    }

    pub(super) fn highlights(
        &self,
        p: lsp::DocumentHighlightParams,
    ) -> Result<Option<Vec<lsp::DocumentHighlight>>, Later> {
        let pos = p.text_document_position_params;
        let Some(place) = self.current(&pos.text_document.uri)? else {
            return Ok(None);
        };
        let Some(q) = place.a.query() else { return Ok(None) };
        let ix = LineIndex::new(place.text);
        let refs = q.highlights(place.module, Self::offset(&place, pos.position));
        Ok(Some(
            refs.into_iter()
                .filter(|r| r.location.span.end as usize <= place.text.len())
                .map(|r| lsp::DocumentHighlight {
                    range: ix.range(r.location.span),
                    kind: Some(if r.definition {
                        lsp::DocumentHighlightKind::WRITE
                    } else {
                        lsp::DocumentHighlightKind::READ
                    }),
                })
                .collect(),
        ))
    }

    pub(super) fn prepare_rename(&self, p: lsp::TextDocumentPositionParams) -> Answer<lsp::PrepareRenameResponse> {
        let Some(place) = self.current(&p.text_document.uri)? else {
            return Ok(Ok(None));
        };
        let Some(q) = place.a.query() else { return Ok(Ok(None)) };
        match q.prepare_rename(place.module, Self::offset(&place, p.position)) {
            Ok(Some((span, name))) => Ok(Ok(Some(lsp::PrepareRenameResponse::RangeWithPlaceholder {
                range: LineIndex::new(place.text).range(span),
                placeholder: name,
            }))),
            Ok(None) => Ok(Ok(None)),
            Err(e) => Ok(Err((ErrorCode::RequestFailed, e.0))),
        }
    }

    pub(super) fn rename(&self, p: lsp::RenameParams) -> Answer<lsp::WorkspaceEdit> {
        let pos = p.text_document_position;
        let Some(place) = self.current(&pos.text_document.uri)? else {
            return Ok(Ok(None));
        };
        let Some(q) = place.a.query() else { return Ok(Ok(None)) };
        match q.rename(place.module, Self::offset(&place, pos.position), &p.new_name) {
            Ok(edits) => Ok(Ok(Some(self.workspace_edit(&place, edits)?))),
            Err(e) => {
                let code = if e.0.contains("not a valid new name") {
                    ErrorCode::InvalidParams
                } else {
                    ErrorCode::RequestFailed
                };
                Ok(Err((code, e.0)))
            }
        }
    }

    pub(super) fn type_definition(
        &self,
        p: lsp::TextDocumentPositionParams,
        implementation: bool,
    ) -> Result<Option<lsp::GotoDefinitionResponse>, Later> {
        let Some(place) = self.place(&p.text_document.uri)? else {
            return Ok(None);
        };
        let Some(offset) = Self::old_offset(&place, p.position) else {
            return place.later();
        };
        let Some(q) = place.a.query() else { return Ok(None) };
        let found = if implementation {
            q.implementations_at(place.module, offset)
        } else {
            q.type_definition_at(place.module, offset)
        };
        let locations = self.locations(&place, found)?;
        Ok((!locations.is_empty()).then_some(lsp::GotoDefinitionResponse::Array(locations)))
    }

    pub(super) fn signature_help(&self, p: lsp::SignatureHelpParams) -> Result<Option<lsp::SignatureHelp>, Later> {
        let pos = p.text_document_position_params;
        let Some(place) = self.current(&pos.text_document.uri)? else {
            return Ok(None);
        };
        let Some(q) = place.a.query() else { return Ok(None) };
        let Some(s) = q.signature_help(place.module, Self::offset(&place, pos.position)) else {
            return Ok(None);
        };
        let units = |byte: u32| s.label[..byte as usize].encode_utf16().count() as u32;
        let params = s
            .params
            .iter()
            .map(|x| lsp::ParameterInformation {
                label: lsp::ParameterLabel::LabelOffsets([units(x.start), units(x.end)]),
                documentation: None,
            })
            .collect();
        Ok(Some(lsp::SignatureHelp {
            signatures: vec![lsp::SignatureInformation {
                label: s.label.clone(),
                documentation: s.documentation.map(markdown_doc),
                parameters: Some(params),
                active_parameter: s.active,
            }],
            active_signature: Some(0),
            active_parameter: s.active,
        }))
    }

    pub(super) fn code_actions(&self, p: lsp::CodeActionParams) -> Result<Option<lsp::CodeActionResponse>, Later> {
        let Some(place) = self.current(&p.text_document.uri)? else {
            return Ok(None);
        };
        let Some(q) = place.a.query() else { return Ok(None) };
        let ix = LineIndex::new(place.text);
        let range = Span::new(ix.offset(p.range.start), ix.offset(p.range.end));
        let m = place.module;
        let overlaps = |s: Span| s.start <= range.end && range.start <= s.end;
        let mut out: Vec<lsp::CodeActionOrCommand> = Vec::new();
        let mut push = |fix: query::CodeFix, diagnostic: Option<lsp::Diagnostic>, kind: lsp::CodeActionKind| -> Result<(), Later> {
            out.push(lsp::CodeActionOrCommand::CodeAction(lsp::CodeAction {
                title: fix.title,
                kind: Some(kind),
                diagnostics: diagnostic.map(|d| vec![d]),
                edit: Some(self.workspace_edit(&place, fix.edits)?),
                is_preferred: Some(fix.preferred),
                ..Default::default()
            }));
            Ok(())
        };
        for d in &place.a.checked.diags {
            if d.source != Some(place.source) || !overlaps(d.span) {
                continue;
            }
            let diagnostic = Some(self.lsp_diagnostic(place.a, d));
            match d.code {
                ilang_check::code::UNKNOWN_NAME => {
                    for fix in q.missing_import_fixes(m, d.span) {
                        push(fix, diagnostic.clone(), lsp::CodeActionKind::QUICKFIX)?;
                    }
                }
                ilang_check::code::UNUSED_BINDING => {
                    if let Some(fix) = q.remove_binding_fix(m, d.span) {
                        push(fix, diagnostic.clone(), lsp::CodeActionKind::QUICKFIX)?;
                    }
                }
                ilang_check::code::CASE_NOT_COVERED => {
                    if let Some(c) = q.missing_clauses(m, d.span) {
                        let fix = query::CodeFix {
                            title: format!("Add the missing clauses: {}", c.patterns.join(", ")),
                            edits: vec![c.edit],
                            preferred: true,
                        };
                        push(fix, diagnostic.clone(), lsp::CodeActionKind::QUICKFIX)?;
                    }
                }
                _ => {}
            }
        }
        for (item, path) in q.unused_imports(m) {
            if overlaps(item) {
                let fix = query::CodeFix {
                    title: format!("Remove the unused import `{path}`"),
                    edits: vec![q.remove_import_edit(m, item)],
                    preferred: false,
                };
                push(fix, None, lsp::CodeActionKind::QUICKFIX)?;
            }
        }
        if let Some(fix) = q.annotation_fix(m, range.start) {
            push(fix, None, lsp::CodeActionKind::REFACTOR_REWRITE)?;
        }
        Ok(Some(out))
    }

    pub(super) fn inlay_hints(&self, p: lsp::InlayHintParams) -> Result<Option<Vec<lsp::InlayHint>>, Later> {
        let Some(place) = self.current(&p.text_document.uri)? else {
            return Ok(None);
        };
        let Some(q) = place.a.query() else { return Ok(None) };
        let ix = LineIndex::new(place.text);
        let range = Span::new(ix.offset(p.range.start), ix.offset(p.range.end));
        Ok(Some(
            q.inlay_hints(place.module, range)
                .into_iter()
                .map(|h| lsp::InlayHint {
                    position: ix.position(h.offset),
                    label: lsp::InlayHintLabel::String(h.label),
                    kind: Some(if h.parameter {
                        lsp::InlayHintKind::PARAMETER
                    } else {
                        lsp::InlayHintKind::TYPE
                    }),
                    text_edits: None,
                    tooltip: None,
                    padding_left: None,
                    padding_right: None,
                    data: None,
                })
                .collect(),
        ))
    }

    pub(super) fn folding_ranges(&self, p: lsp::FoldingRangeParams) -> Result<Option<Vec<lsp::FoldingRange>>, Later> {
        let Some(place) = self.current(&p.text_document.uri)? else {
            return Ok(None);
        };
        let Some(q) = place.a.query() else { return Ok(None) };
        let ix = LineIndex::new(place.text);
        let mut out: Vec<lsp::FoldingRange> = Vec::new();
        for f in q.folding_ranges(place.module) {
            let r = ix.range(f.span);
            if r.end.line <= r.start.line || out.iter().any(|x| x.start_line == r.start.line) {
                continue;
            }
            out.push(lsp::FoldingRange {
                start_line: r.start.line,
                start_character: Some(r.start.character),
                end_line: r.end.line,
                end_character: Some(r.end.character),
                kind: f.imports.then_some(lsp::FoldingRangeKind::Imports),
                collapsed_text: None,
            });
        }
        Ok(Some(out))
    }

    pub(super) fn selection_ranges(
        &self,
        p: lsp::SelectionRangeParams,
    ) -> Result<Option<Vec<lsp::SelectionRange>>, Later> {
        let Some(place) = self.current(&p.text_document.uri)? else {
            return Ok(None);
        };
        let Some(q) = place.a.query() else { return Ok(None) };
        let ix = LineIndex::new(place.text);
        let mut out = Vec::new();
        for pos in p.positions {
            let spans = q.selection_ranges(place.module, ix.offset(pos));
            let mut range: Option<lsp::SelectionRange> = None;
            for s in spans.iter().rev() {
                range = Some(lsp::SelectionRange {
                    range: ix.range(*s),
                    parent: range.map(Box::new),
                });
            }
            out.push(range.unwrap_or(lsp::SelectionRange {
                range: lsp::Range::new(pos, pos),
                parent: None,
            }));
        }
        Ok(Some(out))
    }

    pub(super) fn document_links(&self, p: lsp::DocumentLinkParams) -> Result<Option<Vec<lsp::DocumentLink>>, Later> {
        let Some(place) = self.current(&p.text_document.uri)? else {
            return Ok(None);
        };
        let Some(q) = place.a.query() else { return Ok(None) };
        let ix = LineIndex::new(place.text);
        let mut out = Vec::new();
        for l in q.document_links(place.module) {
            let Some(path) = place.a.paths.get(&l.source) else {
                continue;
            };
            if self.library_dir().is_some_and(|d| path.starts_with(d)) {
                write_library(place.a);
            }
            out.push(lsp::DocumentLink {
                range: ix.range(l.span),
                target: self.uri_of(path),
                tooltip: Some(format!("module {}", q.program().module(l.target).path)),
                data: None,
            });
        }
        Ok(Some(out))
    }

    /// The definitions of the checked packages whose names hold the query.
    pub(super) fn workspace_symbols(&self, p: lsp::WorkspaceSymbolParams) -> Option<lsp::WorkspaceSymbolResponse> {
        let mut out = Vec::new();
        let mut roots: Vec<&PathBuf> = self.analyses.keys().collect();
        roots.sort();
        let mut seen = HashSet::new();
        for root in roots {
            let a = &self.analyses[root];
            let Some(q) = a.query() else { continue };
            for s in q.workspace_symbols(&p.query) {
                let Some(path) = a.paths.get(&s.location.source) else {
                    continue;
                };
                if !seen.insert((path.clone(), s.location.span)) {
                    continue;
                }
                let Some(uri) = self.uri_of(path) else { continue };
                let Some(range) = self.range_now(a, s.location.source, s.location.span) else {
                    continue;
                };
                #[allow(deprecated)]
                out.push(lsp::SymbolInformation {
                    name: s.name,
                    kind: symbol_kind(s.kind),
                    tags: None,
                    deprecated: None,
                    location: lsp::Location { uri, range },
                    container_name: Some(s.container),
                });
            }
        }
        Some(lsp::WorkspaceSymbolResponse::Flat(out))
    }

    /// `completionItem/resolve`: the documentation of the item.
    pub(super) fn resolve_completion(&self, mut item: lsp::CompletionItem) -> lsp::CompletionItem {
        let Some(data) = item.data.take() else {
            return item;
        };
        let uri = data["uri"].as_str().and_then(|u| u.parse::<lsp::Uri>().ok());
        let path = data["module"].as_str();
        let found = uri.zip(path).and_then(|(uri, path)| {
            let file = normalize(&uri::to_path(&uri)?);
            let a = self.analyses.get(&package_root(&file)?)?;
            a.query()?.documentation_of(path, &item.label)
        });
        if let Some(doc) = found {
            item.documentation = Some(markdown_doc(doc));
        }
        item
    }

    /// The LSP form of a problem of a check.
    pub(super) fn lsp_diagnostic(&self, a: &Analysis, d: &ilang_package::Diagnostic) -> lsp::Diagnostic {
        let range = match d.source {
            Some(s) if a.paths.contains_key(&s) => LineIndex::new(a.text(s)).range(d.span),
            _ => lsp::Range::default(),
        };
        let data = match d.code {
            ilang_check::code::UNKNOWN_NAME => Some(serde_json::json!({ "fix": "import" })),
            ilang_check::code::UNUSED_BINDING => Some(serde_json::json!({ "fix": "remove" })),
            ilang_check::code::CASE_NOT_COVERED => Some(serde_json::json!({ "fix": "clauses" })),
            _ => None,
        };
        lsp::Diagnostic {
            range,
            severity: Some(match d.severity {
                Severity::Error => lsp::DiagnosticSeverity::ERROR,
                Severity::Warning => lsp::DiagnosticSeverity::WARNING,
                Severity::Note => lsp::DiagnosticSeverity::INFORMATION,
            }),
            code: Some(lsp::NumberOrString::String(d.code.to_string())),
            source: Some("i".to_string()),
            message: d.message.clone(),
            related_information: (!d.related.is_empty()).then(|| {
                d.related
                    .iter()
                    .filter_map(|note| {
                        let path = a.paths.get(&note.source)?;
                        let uri = self.uri_of(path)?;
                        Some(lsp::DiagnosticRelatedInformation {
                            location: lsp::Location {
                                uri,
                                range: LineIndex::new(a.text(note.source)).range(note.span),
                            },
                            message: note.message.clone(),
                        })
                    })
                    .collect()
            }),
            tags: (d.code == ilang_check::code::UNUSED_BINDING).then(|| vec![lsp::DiagnosticTag::UNNECESSARY]),
            data,
            ..Default::default()
        }
    }
}

pub(super) fn markdown_doc(doc: String) -> lsp::Documentation {
    lsp::Documentation::MarkupContent(lsp::MarkupContent {
        kind: lsp::MarkupKind::Markdown,
        value: doc,
    })
}

pub(super) fn symbol_kind(k: SymbolKind) -> lsp::SymbolKind {
    match k {
        SymbolKind::Function => lsp::SymbolKind::FUNCTION,
        SymbolKind::Value => lsp::SymbolKind::CONSTANT,
        SymbolKind::Type => lsp::SymbolKind::CLASS,
        SymbolKind::Constructor => lsp::SymbolKind::CONSTRUCTOR,
    }
}

/// The completion items of a request, with their documentation, edits, and
/// the data that `completionItem/resolve` reads.
pub(super) fn completion_items(
    q: &Query,
    uri: &lsp::Uri,
    text: &str,
    items: Vec<query::Completion>,
) -> lsp::CompletionResponse {
    let ix = LineIndex::new(text);
    let out = items
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let edits: Vec<lsp::TextEdit> = c
                .edits
                .iter()
                .filter(|e| e.span.end as usize <= text.len())
                .map(|e| lsp::TextEdit {
                    range: ix.range(e.span),
                    new_text: e.text.clone(),
                })
                .collect();
            let insert = c.insert.clone().unwrap_or_else(|| c.label.clone());
            let text_edit = c.replace.filter(|s| s.end as usize <= text.len()).map(|s| {
                lsp::CompletionTextEdit::Edit(lsp::TextEdit {
                    range: ix.range(s),
                    new_text: insert.clone(),
                })
            });
            let data = match (&c.defined_in, &c.documentation) {
                (Some(module), None) => Some(serde_json::json!({ "uri": uri.as_str(), "module": module })),
                _ => None,
            };
            let _ = q;
            lsp::CompletionItem {
                kind: Some(completion_kind(c.kind)),
                detail: c.detail,
                documentation: c.documentation.map(markdown_doc),
                insert_text: (text_edit.is_none() && c.insert.is_some()).then_some(insert),
                insert_text_format: c.snippet.then_some(lsp::InsertTextFormat::SNIPPET),
                text_edit,
                additional_text_edits: (!edits.is_empty()).then_some(edits),
                sort_text: Some(format!("{}{:05}", if c.preferred { 0 } else { 1 }, i)),
                filter_text: None,
                data,
                label: c.label,
                ..Default::default()
            }
        })
        .collect();
    lsp::CompletionResponse::Array(out)
}
