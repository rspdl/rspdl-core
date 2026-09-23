use rspdl_compiler::{Source, compile_ko_files};
use rspdl_ko::{
    DocumentAst, FrontmatterRefAst, HandlerKindAst, LayoutElementAst, SameScreenHandlerAst,
    ScreenPathAst, Span, format_document, parse,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

use crate::{SUPPORTED_LOCALE, WIRE_SCHEMA_VERSION};

pub const EDIT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditRequest {
    pub schema_version: u32,
    pub locale: String,
    pub source: EditSource,
    pub expected_source_hash: String,
    pub edit: EditOperation,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditSource {
    pub path: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditOperation {
    Insert {
        screen_id: String,
        parent_element_id: Option<String>,
        slot: EditSlot,
        before_element_id: Option<String>,
        element: EditElement,
    },
    Delete {
        screen_id: String,
        element_id: String,
    },
    Move {
        screen_id: String,
        element_id: String,
        parent_element_id: Option<String>,
        slot: EditSlot,
        before_element_id: Option<String>,
    },
    Update {
        screen_id: String,
        element_id: String,
        patch: EditPatch,
    },
    Connect {
        source_screen_id: String,
        source_element_id: String,
        target_screen_id: Option<String>,
        outcome_id: Option<String>,
        handler: Option<EditHandler>,
        label: Option<String>,
    },
    Disconnect {
        source_screen_id: String,
        source_element_id: String,
        target_screen_id: Option<String>,
        outcome_id: Option<String>,
        handler: Option<EditHandler>,
        label: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EditHandler {
    pub kind: EditHandlerKind,
    pub id: String,
    pub content: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EditHandlerKind {
    State,
    Message,
    Popup,
    Loading,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditSlot {
    Root,
    Children,
    Inputs,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EditElement {
    Header {
        id: String,
    },
    Section {
        id: String,
    },
    Form {
        id: String,
    },
    Heading {
        id: String,
        text: String,
    },
    Input {
        id: String,
        field_id: String,
    },
    List {
        id: String,
        model_id: String,
        field_ids: Vec<String>,
    },
    Button {
        id: String,
        name: String,
        action_id: Option<String>,
    },
    Placeholder {
        id: String,
        text: String,
    },
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditPatch {
    pub text: Option<String>,
    pub field_id: Option<String>,
    pub model_id: Option<String>,
    pub field_ids: Option<Vec<String>>,
    pub name: Option<String>,
    pub action_id: Option<String>,
    #[serde(default)]
    pub clear_action: bool,
}

#[derive(Debug, Serialize)]
pub struct EditResponse {
    pub schema_version: u32,
    pub wire_schema_version: u32,
    pub rspdl_version: &'static str,
    pub locale: &'static str,
    pub source_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_source_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_text: Option<String>,
    pub outcome: EditOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compilation: Option<serde_json::Value>,
    pub tombstones: Vec<String>,
    pub id_remap: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum EditOutcome {
    Applied,
    Rejected { code: &'static str, reason: String },
}

pub fn source_hash(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub fn edit(request: EditRequest) -> EditResponse {
    let actual_hash = source_hash(&request.source.text);
    let reject = |code, reason| EditResponse {
        schema_version: EDIT_SCHEMA_VERSION,
        wire_schema_version: WIRE_SCHEMA_VERSION,
        rspdl_version: env!("CARGO_PKG_VERSION"),
        locale: SUPPORTED_LOCALE,
        source_hash: actual_hash.clone(),
        candidate_source_hash: None,
        candidate_text: None,
        outcome: EditOutcome::Rejected { code, reason },
        compilation: None,
        tombstones: vec![],
        id_remap: BTreeMap::new(),
    };
    if request.schema_version != EDIT_SCHEMA_VERSION {
        return reject("RSPDL-EDIT-001", "unsupported edit schema version".into());
    }
    if request.locale != SUPPORTED_LOCALE {
        return reject("RSPDL-EDIT-002", "unsupported locale".into());
    }
    if request.expected_source_hash != actual_hash {
        return reject("RSPDL-EDIT-STALE", "source hash does not match".into());
    }
    let parsed = parse(&request.source.text);
    if !parsed.diagnostics.is_empty() {
        return reject("RSPDL-EDIT-PARSE", "source has parse diagnostics".into());
    }
    let Some(mut document) = parsed.document else {
        return reject("RSPDL-EDIT-PARSE", "source does not parse".into());
    };
    let tombstones = deletion_tombstones(&document, &request.edit).unwrap_or_default();
    let outcome = apply(&mut document, &request.edit);
    if let Err((code, reason)) = outcome {
        return reject(code, reason);
    }
    let candidate = match format_document(&document) {
        Ok(text) => text,
        Err(error) => return reject("RSPDL-EDIT-FORMAT", error.to_string()),
    };
    let candidate_hash = source_hash(&candidate);
    if candidate_hash == actual_hash {
        return reject(
            "RSPDL-EDIT-UNCHANGED",
            "edit produced no source change".into(),
        );
    }
    let compilation = serde_json::to_value(compile_ko_files(vec![Source::new(
        request.source.path,
        candidate.clone(),
    )]))
    .expect("compiler response serializes");
    EditResponse {
        schema_version: EDIT_SCHEMA_VERSION,
        wire_schema_version: WIRE_SCHEMA_VERSION,
        rspdl_version: env!("CARGO_PKG_VERSION"),
        locale: SUPPORTED_LOCALE,
        source_hash: actual_hash,
        candidate_source_hash: Some(candidate_hash),
        candidate_text: Some(candidate),
        outcome: EditOutcome::Applied,
        compilation: Some(compilation),
        tombstones,
        id_remap: BTreeMap::new(),
    }
}

type EditResult = Result<(), (&'static str, String)>;

fn apply(document: &mut DocumentAst, operation: &EditOperation) -> EditResult {
    if document.frontmatter.is_none() {
        return Err((
            "RSPDL-EDIT-NO-FRONTMATTER",
            "document has no frontmatter".into(),
        ));
    }
    match operation {
        EditOperation::Connect {
            source_screen_id,
            source_element_id,
            target_screen_id,
            outcome_id,
            handler,
            label,
        } => {
            let count = {
                let elements = screen_elements_mut(document, source_screen_id)?;
                count_elements(elements, source_element_id)
            };
            if count != 1 {
                return Err((
                    if count == 0 {
                        "RSPDL-EDIT-NOT-FOUND"
                    } else {
                        "RSPDL-EDIT-AMBIGUOUS"
                    },
                    format!("source element matched {count} times"),
                ));
            }
            if target_screen_id.is_some() == handler.is_some() {
                return Err((
                    "RSPDL-EDIT-INVALID",
                    "connect requires exactly one target_screen_id or handler".into(),
                ));
            }
            if let Some(target) = target_screen_id {
                validate_screen(document, target)?;
            }
            let source_screen_ref = screen_source_reference(document, source_screen_id)?;
            let target_screen_ref = target_screen_id
                .as_deref()
                .map(|target| screen_source_reference(document, target))
                .transpose()?;
            let outcome_ref = outcome_id.as_deref().map(|outcome| {
                normalize_outcome_reference(document, source_screen_id, source_element_id, outcome)
            });
            let duplicate = document
                .frontmatter
                .as_ref()
                .unwrap()
                .paths
                .iter()
                .any(|path| {
                    screen_ref_matches(document, &path.source_screen, source_screen_id)
                        && path.source_element.text == *source_element_id
                        && option_screen_ref_matches(
                            document,
                            path.target_screen.as_ref(),
                            target_screen_id.as_deref(),
                        )
                        && option_outcome_ref_matches(
                            document,
                            source_screen_id,
                            source_element_id,
                            path.outcome.as_ref(),
                            outcome_id.as_deref(),
                        )
                        && edit_handler_matches(path.handler.as_ref(), handler.as_ref())
                        && path.label == *label
                });
            if duplicate {
                return Err(("RSPDL-EDIT-UNCHANGED", "screen path already exists".into()));
            }
            document
                .frontmatter
                .as_mut()
                .unwrap()
                .paths
                .push(ScreenPathAst {
                    id: None,
                    source_screen: reference(&source_screen_ref),
                    source_element: reference(source_element_id),
                    target_screen: target_screen_ref.as_deref().map(reference),
                    outcome: outcome_ref.as_deref().map(reference),
                    handler: handler.as_ref().map(to_handler_ast),
                    label: label.clone(),
                    span: empty_span(),
                });
            Ok(())
        }
        EditOperation::Disconnect {
            source_screen_id,
            source_element_id,
            target_screen_id,
            outcome_id,
            handler,
            label,
        } => {
            if target_screen_id.is_some() == handler.is_some() {
                return Err((
                    "RSPDL-EDIT-INVALID",
                    "disconnect requires exactly one target_screen_id or handler".into(),
                ));
            }
            let matching = document
                .frontmatter
                .as_ref()
                .unwrap()
                .paths
                .iter()
                .filter(|path| {
                    screen_ref_matches(document, &path.source_screen, source_screen_id)
                        && path.source_element.text == *source_element_id
                        && option_screen_ref_matches(
                            document,
                            path.target_screen.as_ref(),
                            target_screen_id.as_deref(),
                        )
                        && option_outcome_ref_matches(
                            document,
                            source_screen_id,
                            source_element_id,
                            path.outcome.as_ref(),
                            outcome_id.as_deref(),
                        )
                        && edit_handler_matches(path.handler.as_ref(), handler.as_ref())
                        && path.label == *label
                })
                .count();
            if matching == 0 {
                return Err(("RSPDL-EDIT-NOT-FOUND", "screen path not found".into()));
            }
            if matching > 1 {
                return Err((
                    "RSPDL-EDIT-AMBIGUOUS",
                    format!("screen path matched {matching} times"),
                ));
            }
            let index = document
                .frontmatter
                .as_ref()
                .unwrap()
                .paths
                .iter()
                .position(|path| {
                    screen_ref_matches(document, &path.source_screen, source_screen_id)
                        && path.source_element.text == *source_element_id
                        && option_screen_ref_matches(
                            document,
                            path.target_screen.as_ref(),
                            target_screen_id.as_deref(),
                        )
                        && option_outcome_ref_matches(
                            document,
                            source_screen_id,
                            source_element_id,
                            path.outcome.as_ref(),
                            outcome_id.as_deref(),
                        )
                        && edit_handler_matches(path.handler.as_ref(), handler.as_ref())
                        && path.label == *label
                })
                .expect("exactly one matching path was counted");
            document.frontmatter.as_mut().unwrap().paths.remove(index);
            Ok(())
        }
        EditOperation::Insert {
            screen_id,
            parent_element_id,
            slot,
            before_element_id,
            element,
        } => {
            let element = normalize_edit_element(document, element.clone())?;
            let elements = screen_elements_mut(document, screen_id)?;
            if count_elements(elements, element_id_of_edit(&element)) != 0 {
                return Err((
                    "RSPDL-EDIT-AMBIGUOUS",
                    "inserted element id already exists".into(),
                ));
            }
            validate_insert_address(
                elements,
                parent_element_id.as_deref(),
                *slot,
                before_element_id.as_deref(),
            )?;
            insert(
                elements,
                parent_element_id.as_deref(),
                *slot,
                before_element_id.as_deref(),
                to_ast(element),
            )
        }
        EditOperation::Delete {
            screen_id,
            element_id,
        } => {
            let elements = screen_elements_mut(document, screen_id)?;
            let count = count_elements(elements, element_id);
            if count != 1 {
                return Err((
                    if count == 0 {
                        "RSPDL-EDIT-NOT-FOUND"
                    } else {
                        "RSPDL-EDIT-AMBIGUOUS"
                    },
                    format!("element {element_id} matched {count} times"),
                ));
            }
            let found = remove_element(elements, element_id);
            found.map(|_| ()).ok_or((
                "RSPDL-EDIT-NOT-FOUND",
                format!("element {element_id} not found"),
            ))
        }
        EditOperation::Move {
            screen_id,
            element_id,
            parent_element_id,
            slot,
            before_element_id,
        } => {
            let elements = screen_elements_mut(document, screen_id)?;
            let count = count_elements(elements, element_id);
            if count != 1 {
                return Err((
                    if count == 0 {
                        "RSPDL-EDIT-NOT-FOUND"
                    } else {
                        "RSPDL-EDIT-AMBIGUOUS"
                    },
                    format!("element {element_id} matched {count} times"),
                ));
            }
            if before_element_id.as_deref() == Some(element_id) {
                return Err((
                    "RSPDL-EDIT-UNCHANGED",
                    "cannot move an element before itself".into(),
                ));
            }
            if let Some(parent) = parent_element_id.as_deref() {
                require_unique(elements, parent, "parent element")?;
                let moving = find_element(elements, element_id).expect("unique element exists");
                if crate::edit::element_id(moving) == Some(parent)
                    || count_elements_in(moving, parent) != 0
                {
                    return Err((
                        "RSPDL-EDIT-SLOT",
                        "cannot move an element into itself or its descendant".into(),
                    ));
                }
            }
            if let Some(before) = before_element_id.as_deref() {
                require_unique(elements, before, "before element")?;
                let moving = find_element(elements, element_id).expect("unique element exists");
                if count_elements_in(moving, before) != 0 {
                    return Err((
                        "RSPDL-EDIT-SLOT",
                        "cannot place an element relative to its descendant".into(),
                    ));
                }
            }
            let element = remove_element(elements, element_id).ok_or((
                "RSPDL-EDIT-NOT-FOUND",
                format!("element {element_id} not found"),
            ))?;
            insert(
                elements,
                parent_element_id.as_deref(),
                *slot,
                before_element_id.as_deref(),
                element,
            )
        }
        EditOperation::Update {
            screen_id,
            element_id,
            patch,
        } => {
            let patch = normalize_patch(document, patch.clone())?;
            let elements = screen_elements_mut(document, screen_id)?;
            let count = count_elements(elements, element_id);
            if count != 1 {
                return Err((
                    if count == 0 {
                        "RSPDL-EDIT-NOT-FOUND"
                    } else {
                        "RSPDL-EDIT-AMBIGUOUS"
                    },
                    format!("element {element_id} matched {count} times"),
                ));
            }
            update_element(find_element_mut(elements, element_id).unwrap(), &patch)
        }
    }
}

fn canonical_parts<'a>(
    document: &DocumentAst,
    value: &'a str,
    expected: usize,
) -> Result<Vec<&'a str>, (&'static str, String)> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() == 1 {
        return Ok(parts);
    }
    if parts.len() != expected || parts[0] != document.module.declaration.id {
        return Err((
            "RSPDL-EDIT-PATCH",
            format!("reference {value} is outside this source module"),
        ));
    }
    Ok(parts)
}

fn normalize_edit_element(
    document: &DocumentAst,
    element: EditElement,
) -> Result<EditElement, (&'static str, String)> {
    Ok(match element {
        EditElement::Input { id, field_id } => EditElement::Input {
            id,
            field_id: normalize_field(document, &field_id)?,
        },
        EditElement::List {
            id,
            model_id,
            field_ids,
        } => EditElement::List {
            id,
            model_id: normalize_decl_ref(document, &model_id, "model")?,
            field_ids: field_ids
                .iter()
                .map(|value| normalize_field(document, value))
                .collect::<Result<_, _>>()?,
        },
        EditElement::Button {
            id,
            name,
            action_id,
        } => EditElement::Button {
            id,
            name,
            action_id: action_id
                .map(|value| normalize_decl_ref(document, &value, "action"))
                .transpose()?,
        },
        other => other,
    })
}

fn normalize_patch(
    document: &DocumentAst,
    mut patch: EditPatch,
) -> Result<EditPatch, (&'static str, String)> {
    patch.field_id = patch
        .field_id
        .as_deref()
        .map(|value| normalize_field(document, value))
        .transpose()?;
    patch.model_id = patch
        .model_id
        .as_deref()
        .map(|value| normalize_decl_ref(document, value, "model"))
        .transpose()?;
    patch.field_ids = patch
        .field_ids
        .as_ref()
        .map(|values| {
            values
                .iter()
                .map(|value| normalize_field(document, value))
                .collect()
        })
        .transpose()?;
    patch.action_id = patch
        .action_id
        .as_deref()
        .map(|value| normalize_decl_ref(document, value, "action"))
        .transpose()?;
    Ok(patch)
}

fn normalize_decl_ref(
    document: &DocumentAst,
    value: &str,
    kind: &str,
) -> Result<String, (&'static str, String)> {
    let parts = canonical_parts(document, value, 2)?;
    let local = *parts.last().unwrap();
    let exists = document
        .declarations
        .iter()
        .any(|declaration| match (kind, declaration) {
            ("model", rspdl_ko::DeclarationAst::DataModel(model)) => model.declaration.id == local,
            ("action", rspdl_ko::DeclarationAst::Action(action)) => action.declaration.id == local,
            _ => false,
        });
    if parts.len() > 1 && !exists {
        return Err((
            "RSPDL-EDIT-PATCH",
            format!("unknown {kind} reference {value}"),
        ));
    }
    Ok(local.to_owned())
}

fn normalize_field(document: &DocumentAst, value: &str) -> Result<String, (&'static str, String)> {
    let parts = canonical_parts(document, value, 3)?;
    if parts.len() == 1 {
        return Ok(value.to_owned());
    }
    let exists = document.declarations.iter().any(|declaration| matches!(declaration,
        rspdl_ko::DeclarationAst::DataModel(model) if model.declaration.id == parts[1] && model.fields.iter().any(|field| field.declaration.id == parts[2])));
    if !exists {
        return Err((
            "RSPDL-EDIT-PATCH",
            format!("unknown field reference {value}"),
        ));
    }
    Ok(parts[2].to_owned())
}

fn screen_elements_mut<'a>(
    document: &'a mut DocumentAst,
    screen_id: &str,
) -> Result<&'a mut Vec<LayoutElementAst>, (&'static str, String)> {
    let module_id = &document.module.declaration.id;
    let (name, local_id) = document
        .declarations
        .iter()
        .find_map(|declaration| match declaration {
            rspdl_ko::DeclarationAst::Screen(screen)
                if screen_id == format!("{module_id}.{}", screen.declaration.id) =>
            {
                Some((
                    screen.declaration.name.clone(),
                    screen.declaration.id.clone(),
                ))
            }
            _ => None,
        })
        .ok_or((
            "RSPDL-EDIT-SCREEN-NOT-FOUND",
            format!("screen {screen_id} not found"),
        ))?;
    let frontmatter = document.frontmatter.as_mut().unwrap();
    let mut matches = frontmatter
        .screens
        .iter_mut()
        .filter(|layout| {
            layout.screen.text == name
                || layout.screen.text == local_id
                || layout.screen.text == screen_id
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err((
            if matches.is_empty() {
                "RSPDL-EDIT-SCREEN-NOT-FOUND"
            } else {
                "RSPDL-EDIT-AMBIGUOUS"
            },
            format!("screen {screen_id} layout matched {} times", matches.len()),
        ));
    }
    Ok(&mut matches.pop().unwrap().elements)
}

fn screen_elements<'a>(
    document: &'a DocumentAst,
    screen_id: &str,
) -> Result<&'a Vec<LayoutElementAst>, (&'static str, String)> {
    let module_id = &document.module.declaration.id;
    let (name, local_id) = document
        .declarations
        .iter()
        .find_map(|declaration| match declaration {
            rspdl_ko::DeclarationAst::Screen(screen)
                if screen_id == format!("{module_id}.{}", screen.declaration.id) =>
            {
                Some((
                    screen.declaration.name.clone(),
                    screen.declaration.id.clone(),
                ))
            }
            _ => None,
        })
        .ok_or((
            "RSPDL-EDIT-SCREEN-NOT-FOUND",
            format!("screen {screen_id} not found"),
        ))?;
    let matches = document
        .frontmatter
        .as_ref()
        .unwrap()
        .screens
        .iter()
        .filter(|layout| {
            layout.screen.text == name
                || layout.screen.text == local_id
                || layout.screen.text == screen_id
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [layout] => Ok(&layout.elements),
        [] => Err((
            "RSPDL-EDIT-SCREEN-NOT-FOUND",
            format!("screen {screen_id} has no layout"),
        )),
        _ => Err((
            "RSPDL-EDIT-AMBIGUOUS",
            format!("screen {screen_id} layout matched {} times", matches.len()),
        )),
    }
}

fn deletion_tombstones(
    document: &DocumentAst,
    operation: &EditOperation,
) -> Result<Vec<String>, (&'static str, String)> {
    let EditOperation::Delete {
        screen_id,
        element_id,
    } = operation
    else {
        return Ok(vec![]);
    };
    let elements = screen_elements(document, screen_id)?;
    let Some(element) = find_element(elements, element_id) else {
        return Ok(vec![]);
    };
    let mut ids = Vec::new();
    collect_ids(element, &mut ids);
    Ok(ids)
}

fn collect_ids(element: &LayoutElementAst, ids: &mut Vec<String>) {
    if let Some(id) = element_id(element) {
        ids.push(id.to_owned());
    }
    if let Some(children) = child_vec(element) {
        for child in children {
            collect_ids(child, ids);
        }
    }
}

fn validate_screen(document: &DocumentAst, screen_id: &str) -> EditResult {
    let expected_prefix = format!("{}.", document.module.declaration.id);
    let Some(local_id) = screen_id.strip_prefix(&expected_prefix) else {
        return Err((
            "RSPDL-EDIT-SCREEN-NOT-FOUND",
            format!("screen {screen_id} not found"),
        ));
    };
    let count = document
        .declarations
        .iter()
        .filter(|declaration| {
            matches!(declaration,
        rspdl_ko::DeclarationAst::Screen(screen) if screen.declaration.id == local_id)
        })
        .count();
    match count {
        1.. => Ok(()),
        0 => Err((
            "RSPDL-EDIT-SCREEN-NOT-FOUND",
            format!("screen {screen_id} not found"),
        )),
    }
}

fn validate_insert_address(
    elements: &[LayoutElementAst],
    parent: Option<&str>,
    slot: EditSlot,
    before: Option<&str>,
) -> EditResult {
    if let Some(parent) = parent {
        require_unique(elements, parent, "parent element")?;
    } else if !matches!(slot, EditSlot::Root) {
        return Err(("RSPDL-EDIT-SLOT", "non-root slot needs parent".into()));
    }
    if let Some(before) = before {
        require_unique(elements, before, "before element")?;
    }
    Ok(())
}

fn require_unique(elements: &[LayoutElementAst], id: &str, what: &str) -> EditResult {
    let count = count_elements(elements, id);
    match count {
        1 => Ok(()),
        0 => Err(("RSPDL-EDIT-NOT-FOUND", format!("{what} {id} not found"))),
        _ => Err((
            "RSPDL-EDIT-AMBIGUOUS",
            format!("{what} {id} matched {count} times"),
        )),
    }
}

fn insert(
    elements: &mut Vec<LayoutElementAst>,
    parent: Option<&str>,
    slot: EditSlot,
    before: Option<&str>,
    element: LayoutElementAst,
) -> EditResult {
    let target = if let Some(parent) = parent {
        find_children_mut(elements, parent, slot)?
    } else if matches!(slot, EditSlot::Root) {
        elements
    } else {
        return Err(("RSPDL-EDIT-SLOT", "non-root slot needs parent".into()));
    };
    let index = match before {
        None => target.len(),
        Some(id) => target
            .iter()
            .position(|item| element_id(item) == Some(id))
            .ok_or((
                "RSPDL-EDIT-NOT-FOUND",
                format!("before element {id} not found"),
            ))?,
    };
    target.insert(index, element);
    Ok(())
}

fn find_children_mut<'a>(
    elements: &'a mut Vec<LayoutElementAst>,
    id: &str,
    slot: EditSlot,
) -> Result<&'a mut Vec<LayoutElementAst>, (&'static str, String)> {
    for element in elements {
        if element_id(element) == Some(id) {
            return match (element, slot) {
                (
                    LayoutElementAst::Header { children, .. }
                    | LayoutElementAst::Section { children, .. },
                    EditSlot::Children,
                ) => Ok(children),
                (LayoutElementAst::Form { inputs, .. }, EditSlot::Inputs) => Ok(inputs),
                _ => Err(("RSPDL-EDIT-SLOT", format!("invalid slot for {id}"))),
            };
        }
        if let Some(children) = child_vec_mut(element)
            && let Ok(found) = find_children_mut(children, id, slot)
        {
            return Ok(found);
        }
    }
    Err((
        "RSPDL-EDIT-NOT-FOUND",
        format!("parent element {id} not found"),
    ))
}

fn remove_element(elements: &mut Vec<LayoutElementAst>, id: &str) -> Option<LayoutElementAst> {
    if let Some(index) = elements
        .iter()
        .position(|item| element_id(item) == Some(id))
    {
        return Some(elements.remove(index));
    }
    for element in elements {
        if let Some(children) = child_vec_mut(element)
            && let Some(found) = remove_element(children, id)
        {
            return Some(found);
        }
    }
    None
}

fn count_elements(elements: &[LayoutElementAst], id: &str) -> usize {
    elements
        .iter()
        .map(|element| {
            usize::from(element_id(element) == Some(id))
                + match element {
                    LayoutElementAst::Header { children, .. }
                    | LayoutElementAst::Section { children, .. } => count_elements(children, id),
                    LayoutElementAst::Form { inputs, .. } => count_elements(inputs, id),
                    _ => 0,
                }
        })
        .sum()
}

fn element_id_of_edit(element: &EditElement) -> &str {
    match element {
        EditElement::Header { id }
        | EditElement::Section { id }
        | EditElement::Form { id }
        | EditElement::Heading { id, .. }
        | EditElement::Input { id, .. }
        | EditElement::List { id, .. }
        | EditElement::Button { id, .. }
        | EditElement::Placeholder { id, .. } => id,
    }
}

fn find_element_mut<'a>(
    elements: &'a mut [LayoutElementAst],
    id: &str,
) -> Option<&'a mut LayoutElementAst> {
    for element in elements {
        if element_id(element) == Some(id) {
            return Some(element);
        }
        if let Some(children) = child_vec_mut(element)
            && let Some(found) = find_element_mut(children, id)
        {
            return Some(found);
        }
    }
    None
}

fn find_element<'a>(elements: &'a [LayoutElementAst], id: &str) -> Option<&'a LayoutElementAst> {
    for element in elements {
        if element_id(element) == Some(id) {
            return Some(element);
        }
        if let Some(children) = child_vec(element)
            && let Some(found) = find_element(children, id)
        {
            return Some(found);
        }
    }
    None
}

fn child_vec(element: &LayoutElementAst) -> Option<&Vec<LayoutElementAst>> {
    match element {
        LayoutElementAst::Header { children, .. } | LayoutElementAst::Section { children, .. } => {
            Some(children)
        }
        LayoutElementAst::Form { inputs, .. } => Some(inputs),
        _ => None,
    }
}

fn count_elements_in(element: &LayoutElementAst, id: &str) -> usize {
    child_vec(element).map_or(0, |children| count_elements(children, id))
}

fn child_vec_mut(element: &mut LayoutElementAst) -> Option<&mut Vec<LayoutElementAst>> {
    match element {
        LayoutElementAst::Header { children, .. } | LayoutElementAst::Section { children, .. } => {
            Some(children)
        }
        LayoutElementAst::Form { inputs, .. } => Some(inputs),
        _ => None,
    }
}
fn element_id(element: &LayoutElementAst) -> Option<&str> {
    match element {
        LayoutElementAst::Header { id, .. }
        | LayoutElementAst::Section { id, .. }
        | LayoutElementAst::Heading { id, .. }
        | LayoutElementAst::Form { id, .. }
        | LayoutElementAst::Input { id, .. }
        | LayoutElementAst::List { id, .. }
        | LayoutElementAst::Placeholder { id, .. } => id.as_deref(),
        LayoutElementAst::Button { id, .. } => Some(id),
    }
}

fn update_element(element: &mut LayoutElementAst, patch: &EditPatch) -> EditResult {
    if patch.text.is_none()
        && patch.field_id.is_none()
        && patch.model_id.is_none()
        && patch.field_ids.is_none()
        && patch.name.is_none()
        && patch.action_id.is_none()
        && !patch.clear_action
    {
        return Err(("RSPDL-EDIT-UNCHANGED", "update patch is empty".into()));
    }
    match element {
        LayoutElementAst::Heading { text, .. } | LayoutElementAst::Placeholder { text, .. } => {
            if patch.field_id.is_some()
                || patch.model_id.is_some()
                || patch.field_ids.is_some()
                || patch.name.is_some()
                || patch.action_id.is_some()
                || patch.clear_action
            {
                return invalid_patch();
            }
            let value = patch
                .text
                .as_ref()
                .ok_or_else(|| invalid_patch().unwrap_err())?;
            if text == value {
                return unchanged_patch();
            }
            *text = value.clone();
        }
        LayoutElementAst::Input { field, .. } => {
            if patch.text.is_some()
                || patch.model_id.is_some()
                || patch.field_ids.is_some()
                || patch.name.is_some()
                || patch.action_id.is_some()
                || patch.clear_action
            {
                return invalid_patch();
            }
            let value = patch
                .field_id
                .as_ref()
                .ok_or_else(|| invalid_patch().unwrap_err())?;
            if field.text == *value {
                return unchanged_patch();
            }
            *field = reference(value);
        }
        LayoutElementAst::List { model, fields, .. } => {
            if patch.text.is_some()
                || patch.field_id.is_some()
                || patch.name.is_some()
                || patch.action_id.is_some()
                || patch.clear_action
            {
                return invalid_patch();
            }
            let mut changed = false;
            if let Some(value) = &patch.model_id {
                changed |= model.text != *value;
                *model = reference(value);
            }
            if let Some(values) = &patch.field_ids {
                changed |= fields.iter().map(|field| &field.text).ne(values.iter());
                *fields = values.iter().map(|value| reference(value)).collect();
            }
            if !changed {
                return unchanged_patch();
            }
        }
        LayoutElementAst::Button { name, action, .. } => {
            if patch.text.is_some()
                || patch.field_id.is_some()
                || patch.model_id.is_some()
                || patch.field_ids.is_some()
                || (patch.action_id.is_some() && patch.clear_action)
            {
                return invalid_patch();
            }
            let mut changed = false;
            if let Some(value) = &patch.name {
                changed |= name != value;
                *name = value.clone();
            }
            if let Some(value) = &patch.action_id {
                changed |= action.as_ref().map(|item| &item.text) != Some(value);
                *action = Some(reference(value));
            }
            if patch.clear_action {
                changed |= action.is_some();
                *action = None;
            }
            if !changed {
                return unchanged_patch();
            }
        }
        _ => {
            return Err((
                "RSPDL-EDIT-PATCH",
                "element kind has no editable scalar properties".into(),
            ));
        }
    }
    Ok(())
}

fn invalid_patch() -> EditResult {
    Err((
        "RSPDL-EDIT-PATCH",
        "patch contains properties unsupported by this element kind".into(),
    ))
}
fn unchanged_patch() -> EditResult {
    Err((
        "RSPDL-EDIT-UNCHANGED",
        "update does not change the element".into(),
    ))
}

fn to_ast(element: EditElement) -> LayoutElementAst {
    match element {
        EditElement::Header { id } => LayoutElementAst::Header {
            id: Some(id),
            children: vec![],
            span: empty_span(),
        },
        EditElement::Section { id } => LayoutElementAst::Section {
            id: Some(id),
            children: vec![],
            span: empty_span(),
        },
        EditElement::Form { id } => LayoutElementAst::Form {
            id: Some(id),
            inputs: vec![],
            span: empty_span(),
        },
        EditElement::Heading { id, text } => LayoutElementAst::Heading {
            id: Some(id),
            text,
            span: empty_span(),
        },
        EditElement::Input { id, field_id } => LayoutElementAst::Input {
            id: Some(id),
            field: reference(&field_id),
            span: empty_span(),
        },
        EditElement::List {
            id,
            model_id,
            field_ids,
        } => LayoutElementAst::List {
            id: Some(id),
            model: reference(&model_id),
            fields: field_ids.iter().map(|value| reference(value)).collect(),
            span: empty_span(),
        },
        EditElement::Button {
            id,
            name,
            action_id,
        } => LayoutElementAst::Button {
            id,
            name,
            action: action_id.map(|value| reference(&value)),
            span: empty_span(),
        },
        EditElement::Placeholder { id, text } => LayoutElementAst::Placeholder {
            id: Some(id),
            text,
            span: empty_span(),
        },
    }
}
fn reference(value: &str) -> FrontmatterRefAst {
    FrontmatterRefAst {
        text: value.to_owned(),
        span: empty_span(),
    }
}

fn screen_source_reference(
    document: &DocumentAst,
    canonical: &str,
) -> Result<String, (&'static str, String)> {
    let module_id = &document.module.declaration.id;
    document
        .declarations
        .iter()
        .find_map(|declaration| match declaration {
            rspdl_ko::DeclarationAst::Screen(screen)
                if canonical == format!("{module_id}.{}", screen.declaration.id) =>
            {
                Some(if screen.declaration.name.is_empty() {
                    screen.declaration.id.clone()
                } else {
                    screen.declaration.name.clone()
                })
            }
            _ => None,
        })
        .ok_or((
            "RSPDL-EDIT-SCREEN-NOT-FOUND",
            format!("screen {canonical} not found"),
        ))
}

fn normalize_outcome_reference(
    document: &DocumentAst,
    source_screen_id: &str,
    source_element_id: &str,
    reference: &str,
) -> String {
    let Some(elements) = screen_elements(document, source_screen_id).ok() else {
        return reference.to_owned();
    };
    let Some(LayoutElementAst::Button {
        action: Some(action),
        ..
    }) = find_element(elements, source_element_id)
    else {
        return reference.to_owned();
    };
    let Some(action_id) = canonical_action_id(document, &action.text) else {
        return reference.to_owned();
    };
    document
        .frontmatter
        .as_ref()
        .unwrap()
        .action_outcomes
        .iter()
        .filter(|group| {
            canonical_action_id(document, &group.action.text).as_deref() == Some(&action_id)
        })
        .flat_map(|group| &group.outcomes)
        .find(|outcome| {
            outcome.id == reference || format!("{action_id}.{}", outcome.id) == reference
        })
        .map_or_else(|| reference.to_owned(), |outcome| outcome.id.clone())
}

fn canonical_action_id(document: &DocumentAst, reference: &str) -> Option<String> {
    let module_id = &document.module.declaration.id;
    let matches = document
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            rspdl_ko::DeclarationAst::Action(action) => {
                let canonical = format!("{module_id}.{}", action.declaration.id);
                (reference == action.declaration.name
                    || reference == action.declaration.id
                    || reference == canonical)
                    .then_some(canonical)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [canonical] => Some(canonical.clone()),
        _ => None,
    }
}

fn option_outcome_ref_matches(
    document: &DocumentAst,
    source_screen_id: &str,
    source_element_id: &str,
    value: Option<&FrontmatterRefAst>,
    expected: Option<&str>,
) -> bool {
    match (value, expected) {
        (Some(value), Some(expected)) => {
            normalize_outcome_reference(document, source_screen_id, source_element_id, &value.text)
                == normalize_outcome_reference(
                    document,
                    source_screen_id,
                    source_element_id,
                    expected,
                )
        }
        (None, None) => true,
        _ => false,
    }
}

fn screen_identity_map(document: &DocumentAst) -> Vec<(String, String)> {
    let module_id = &document.module.declaration.id;
    let mut identities = document
        .declarations
        .iter()
        .filter_map(|declaration| match declaration {
            rspdl_ko::DeclarationAst::Screen(screen) => Some((
                screen.declaration.name.clone(),
                format!("{module_id}.{}", screen.declaration.id),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    identities.sort();
    identities.dedup();
    identities
}

fn screen_ref_matches(
    document: &DocumentAst,
    reference: &FrontmatterRefAst,
    canonical: &str,
) -> bool {
    screen_ref_matches_map(&screen_identity_map(document), reference, canonical)
}

fn option_screen_ref_matches(
    document: &DocumentAst,
    reference: Option<&FrontmatterRefAst>,
    expected: Option<&str>,
) -> bool {
    match (reference, expected) {
        (Some(r), Some(e)) => screen_ref_matches(document, r, e),
        (None, None) => true,
        _ => false,
    }
}

fn edit_handler_matches(
    value: Option<&SameScreenHandlerAst>,
    expected: Option<&EditHandler>,
) -> bool {
    match (value, expected) {
        (None, None) => true,
        (Some(v), Some(e)) => {
            v.id == e.id
                && v.content == e.content
                && matches!(
                    (v.kind, e.kind),
                    (HandlerKindAst::State, EditHandlerKind::State)
                        | (HandlerKindAst::Message, EditHandlerKind::Message)
                        | (HandlerKindAst::Popup, EditHandlerKind::Popup)
                        | (HandlerKindAst::Loading, EditHandlerKind::Loading)
                )
        }
        _ => false,
    }
}

fn to_handler_ast(value: &EditHandler) -> SameScreenHandlerAst {
    SameScreenHandlerAst {
        kind: match value.kind {
            EditHandlerKind::State => HandlerKindAst::State,
            EditHandlerKind::Message => HandlerKindAst::Message,
            EditHandlerKind::Popup => HandlerKindAst::Popup,
            EditHandlerKind::Loading => HandlerKindAst::Loading,
        },
        id: value.id.clone(),
        content: value.content.clone(),
        span: empty_span(),
    }
}

fn screen_ref_matches_map(
    identities: &[(String, String)],
    reference: &FrontmatterRefAst,
    canonical: &str,
) -> bool {
    identities.iter().any(|(name, id)| {
        id == canonical
            && (reference.text == *name
                || reference.text == *id
                || reference.text == id.rsplit('.').next().unwrap_or(id))
    })
}
const fn empty_span() -> Span {
    Span { start: 0, end: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = include_str!(
        "../../../conformance/ko-KR/frontmatter-structure/boundary-stable-element-ids/input.rspdl"
    );
    const OUTCOME_SOURCE: &str =
        include_str!("../../../conformance/ko-KR/action-outcomes/normal-lookup/input.rspdl");

    #[test]
    fn every_structured_operation_uses_explicit_ids() {
        let mut document = parse(SOURCE).document.unwrap();
        apply(
            &mut document,
            &EditOperation::Insert {
                screen_id: "catalog.product_form".into(),
                parent_element_id: None,
                slot: EditSlot::Root,
                before_element_id: None,
                element: EditElement::Heading {
                    id: "notice".into(),
                    text: "안내".into(),
                },
            },
        )
        .unwrap();
        apply(
            &mut document,
            &EditOperation::Move {
                screen_id: "catalog.product_form".into(),
                element_id: "notice".into(),
                parent_element_id: Some("content".into()),
                slot: EditSlot::Children,
                before_element_id: Some("product_form".into()),
            },
        )
        .unwrap();
        apply(
            &mut document,
            &EditOperation::Update {
                screen_id: "catalog.product_form".into(),
                element_id: "notice".into(),
                patch: EditPatch {
                    text: Some("필수 입력".into()),
                    ..Default::default()
                },
            },
        )
        .unwrap();
        apply(
            &mut document,
            &EditOperation::Connect {
                source_screen_id: "catalog.product_form".into(),
                source_element_id: "submit".into(),
                target_screen_id: Some("catalog.product_form".into()),
                outcome_id: None,
                handler: None,
                label: Some("저장 뒤".into()),
            },
        )
        .unwrap();
        apply(
            &mut document,
            &EditOperation::Disconnect {
                source_screen_id: "catalog.product_form".into(),
                source_element_id: "submit".into(),
                target_screen_id: Some("catalog.product_form".into()),
                outcome_id: None,
                handler: None,
                label: Some("저장 뒤".into()),
            },
        )
        .unwrap();
        apply(
            &mut document,
            &EditOperation::Delete {
                screen_id: "catalog.product_form".into(),
                element_id: "notice".into(),
            },
        )
        .unwrap();
        let formatted = format_document(&document).unwrap();
        assert!(!formatted.contains("notice"));
        assert!(formatted.contains("id: content"));
    }

    #[test]
    fn rejects_foreign_screen_ids_ambiguous_nested_ids_and_invalid_patches() {
        let mut document = parse(SOURCE).document.unwrap();
        assert_eq!(
            apply(
                &mut document,
                &EditOperation::Update {
                    screen_id: "other.product_form".into(),
                    element_id: "title".into(),
                    patch: EditPatch {
                        text: Some("x".into()),
                        ..Default::default()
                    },
                },
            )
            .unwrap_err()
            .0,
            "RSPDL-EDIT-SCREEN-NOT-FOUND"
        );
        assert_eq!(
            apply(
                &mut document,
                &EditOperation::Update {
                    screen_id: "catalog.product_form".into(),
                    element_id: "name_input".into(),
                    patch: EditPatch {
                        name: Some("ignored".into()),
                        ..Default::default()
                    },
                },
            )
            .unwrap_err()
            .0,
            "RSPDL-EDIT-PATCH"
        );
        let elements = screen_elements_mut(&mut document, "catalog.product_form").unwrap();
        let duplicate = find_element(elements, "title").unwrap().clone();
        if let LayoutElementAst::Section { children, .. } = &mut elements[1] {
            children.push(duplicate);
        }
        assert_eq!(count_elements(elements, "title"), 2);
        assert_eq!(
            apply(
                &mut document,
                &EditOperation::Update {
                    screen_id: "catalog.product_form".into(),
                    element_id: "title".into(),
                    patch: EditPatch {
                        text: Some("x".into()),
                        ..Default::default()
                    },
                },
            )
            .unwrap_err()
            .0,
            "RSPDL-EDIT-AMBIGUOUS"
        );
    }

    #[test]
    fn container_delete_tombstones_all_stable_descendants_and_move_rejects_cycles() {
        let document = parse(SOURCE).document.unwrap();
        let tombstones = deletion_tombstones(
            &document,
            &EditOperation::Delete {
                screen_id: "catalog.product_form".into(),
                element_id: "content".into(),
            },
        )
        .unwrap();
        assert_eq!(
            tombstones,
            [
                "content",
                "product_form",
                "name_input",
                "products",
                "preview",
                "submit"
            ]
        );

        let mut document = document;
        assert_eq!(
            apply(
                &mut document,
                &EditOperation::Move {
                    screen_id: "catalog.product_form".into(),
                    element_id: "content".into(),
                    parent_element_id: Some("product_form".into()),
                    slot: EditSlot::Inputs,
                    before_element_id: None,
                },
            )
            .unwrap_err()
            .0,
            "RSPDL-EDIT-SLOT"
        );
    }

    #[test]
    fn canonical_path_edits_resolve_existing_natural_names_and_preserve_full_field_ids() {
        let source = SOURCE.replacen(
            "\n---\n\n상품(product)",
            "\n흐름:\n  - 출발: 상품 입력 화면.submit\n    도착: 상품 입력 화면\n    설명: \"반복\"\n---\n\n상품(product)",
            1,
        );
        let mut document = parse(&source).document.unwrap();
        let edge = EditOperation::Connect {
            source_screen_id: "catalog.product_form".into(),
            source_element_id: "submit".into(),
            target_screen_id: Some("catalog.product_form".into()),
            outcome_id: None,
            handler: None,
            label: Some("반복".into()),
        };
        assert_eq!(
            apply(&mut document, &edge).unwrap_err().0,
            "RSPDL-EDIT-UNCHANGED"
        );
        apply(
            &mut document,
            &EditOperation::Disconnect {
                source_screen_id: "catalog.product_form".into(),
                source_element_id: "submit".into(),
                target_screen_id: Some("catalog.product_form".into()),
                outcome_id: None,
                handler: None,
                label: Some("반복".into()),
            },
        )
        .unwrap();
        apply(
            &mut document,
            &EditOperation::Update {
                screen_id: "catalog.product_form".into(),
                element_id: "name_input".into(),
                patch: EditPatch {
                    field_id: Some("catalog.product.name".into()),
                    ..Default::default()
                },
            },
        )
        .unwrap();
        let formatted = format_document(&document).unwrap();
        let compiled = compile_ko_files(vec![Source::new("product.rspdl", formatted)]);
        assert!(
            compiled.files[0].diagnostics.is_empty(),
            "{:?}",
            compiled.files[0].diagnostics
        );
        let module = serde_json::to_value(compiled.files[0].module.as_ref().unwrap()).unwrap();
        assert!(module.to_string().contains("catalog.product.name"));
    }

    #[test]
    fn outcome_path_edits_match_every_typed_edge_field() {
        let mut document = parse(SOURCE).document.unwrap();
        for outcome in ["ok", "rejected"] {
            apply(
                &mut document,
                &EditOperation::Connect {
                    source_screen_id: "catalog.product_form".into(),
                    source_element_id: "submit".into(),
                    target_screen_id: Some("catalog.product_form".into()),
                    outcome_id: Some(outcome.into()),
                    handler: None,
                    label: None,
                },
            )
            .unwrap();
        }
        apply(
            &mut document,
            &EditOperation::Disconnect {
                source_screen_id: "catalog.product_form".into(),
                source_element_id: "submit".into(),
                target_screen_id: Some("catalog.product_form".into()),
                outcome_id: Some("ok".into()),
                handler: None,
                label: None,
            },
        )
        .unwrap();
        let paths = &document.frontmatter.unwrap().paths;
        assert!(
            paths
                .iter()
                .any(|path| path.outcome.as_ref().is_some_and(|v| v.text == "rejected"))
        );
        assert!(
            !paths
                .iter()
                .any(|path| path.outcome.as_ref().is_some_and(|v| v.text == "ok"))
        );
    }

    #[test]
    fn canonical_outcome_disconnect_matches_authored_id_and_keeps_other_outcomes() {
        let mut document = parse(OUTCOME_SOURCE).document.unwrap();
        let missing_handler = EditHandler {
            kind: EditHandlerKind::Message,
            id: "missing".into(),
            content: Some("예약을 찾지 못했습니다.".into()),
        };

        assert_eq!(
            apply(
                &mut document,
                &EditOperation::Connect {
                    source_screen_id: "booking.lookup_screen".into(),
                    source_element_id: "lookup".into(),
                    target_screen_id: None,
                    outcome_id: Some("booking.lookup.not_found".into()),
                    handler: Some(missing_handler.clone()),
                    label: None,
                },
            )
            .unwrap_err()
            .0,
            "RSPDL-EDIT-UNCHANGED"
        );

        apply(
            &mut document,
            &EditOperation::Disconnect {
                source_screen_id: "booking.lookup_screen".into(),
                source_element_id: "lookup".into(),
                target_screen_id: None,
                outcome_id: Some("booking.lookup.not_found".into()),
                handler: Some(missing_handler),
                label: None,
            },
        )
        .unwrap();

        let paths = &document.frontmatter.unwrap().paths;
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].outcome.as_ref().unwrap().text, "found");
        assert_eq!(
            paths[0].target_screen.as_ref().unwrap().text,
            "예약 완료 화면"
        );
    }

    #[test]
    fn canonical_outcome_matching_is_scoped_to_the_source_button_action() {
        let source = r#"---
모듈: 작업(flow)
화면:
  작업 화면:
    레이아웃:
      - 버튼: { id: lookup_button, 이름: "조회", 행동: 조회 }
      - 버튼: { id: save_button, 이름: "저장", 행동: 저장 }
행동 결과:
  조회:
    - id: failed
      유형: 실패
  저장:
    - id: failed
      유형: 실패
흐름:
  - 출발: 작업 화면.lookup_button
    결과: failed
    처리: { 종류: 메시지, id: failed }
  - 출발: 작업 화면.save_button
    결과: failed
    처리: { 종류: 메시지, id: failed }
---

항목(item)은 다음 필드들로 구성되어 있다.
    값(value): 필수 문자열

조회(lookup)는 행동이다.
저장(save)은 행동이다.
작업 화면(work_screen)에서는 항목의 값을 조회할 수 있다.
"#;
        let parsed = parse(source);
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let mut document = parsed.document.unwrap();
        let handler = EditHandler {
            kind: EditHandlerKind::Message,
            id: "failed".into(),
            content: None,
        };

        assert_eq!(
            apply(
                &mut document,
                &EditOperation::Disconnect {
                    source_screen_id: "flow.work_screen".into(),
                    source_element_id: "lookup_button".into(),
                    target_screen_id: None,
                    outcome_id: Some("flow.save.failed".into()),
                    handler: Some(handler.clone()),
                    label: None,
                },
            )
            .unwrap_err()
            .0,
            "RSPDL-EDIT-NOT-FOUND"
        );
        assert_eq!(document.frontmatter.as_ref().unwrap().paths.len(), 2);

        apply(
            &mut document,
            &EditOperation::Disconnect {
                source_screen_id: "flow.work_screen".into(),
                source_element_id: "lookup_button".into(),
                target_screen_id: None,
                outcome_id: Some("flow.lookup.failed".into()),
                handler: Some(handler),
                label: None,
            },
        )
        .unwrap();
        let paths = &document.frontmatter.unwrap().paths;
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].source_element.text, "save_button");
    }

    #[test]
    fn canonical_connect_writes_valid_surface_refs_and_returns_its_compilation() {
        let mut document = parse(OUTCOME_SOURCE).document.unwrap();
        document
            .frontmatter
            .as_mut()
            .unwrap()
            .paths
            .retain(|path| path.id.as_deref() != Some("booking.found_path"));
        let source = format_document(&document).unwrap();
        let response = edit(EditRequest {
            schema_version: EDIT_SCHEMA_VERSION,
            locale: SUPPORTED_LOCALE.into(),
            source: EditSource {
                path: "normal-lookup.rspdl".into(),
                text: source.clone(),
            },
            expected_source_hash: source_hash(&source),
            edit: EditOperation::Connect {
                source_screen_id: "booking.lookup_screen".into(),
                source_element_id: "lookup".into(),
                target_screen_id: Some("booking.done_screen".into()),
                outcome_id: Some("booking.lookup.found".into()),
                handler: None,
                label: None,
            },
        });
        assert!(matches!(response.outcome, EditOutcome::Applied));
        let candidate = response.candidate_text.unwrap();
        assert!(candidate.contains("출발: 예약 화면.lookup"));
        assert!(candidate.contains("결과: found"));
        assert!(candidate.contains("도착: 예약 완료 화면"));
        assert!(!candidate.contains("출발: booking.lookup_screen"));
        assert!(!candidate.contains("도착: booking.done_screen"));

        let reparsed = parse(&candidate);
        assert!(
            reparsed.diagnostics.is_empty(),
            "{:?}",
            reparsed.diagnostics
        );
        assert_eq!(
            format_document(&reparsed.document.unwrap()).unwrap(),
            candidate
        );
        let independent = serde_json::to_value(compile_ko_files(vec![Source::new(
            "normal-lookup.rspdl",
            candidate,
        )]))
        .unwrap();
        assert_eq!(response.compilation, Some(independent));
    }

    #[test]
    fn button_action_can_be_cleared_explicitly() {
        let mut document = parse(SOURCE).document.unwrap();
        if let LayoutElementAst::Button { action, .. } = find_element_mut(
            screen_elements_mut(&mut document, "catalog.product_form").unwrap(),
            "submit",
        )
        .unwrap()
        {
            *action = Some(reference("save"));
        }
        apply(
            &mut document,
            &EditOperation::Update {
                screen_id: "catalog.product_form".into(),
                element_id: "submit".into(),
                patch: EditPatch {
                    clear_action: true,
                    ..Default::default()
                },
            },
        )
        .unwrap();
        let formatted = format_document(&document).unwrap();
        assert!(formatted.contains("버튼: { id: submit, 이름: 저장 }"));
    }
}
