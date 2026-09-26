//! Resolved semantic references for downstream navigation.
//!
//! This module deliberately walks typed Semantic IR. Discovering references by
//! looking for JSON keys ending in `_id` would make every consumer reimplement
//! which identifiers are semantic links and which are declarations or ordinary
//! strings.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::{
    ActionInputKind, CanonicalId, CanonicalType, ConstraintOperand, EventInputKind,
    FieldProducerCondition, FieldProducerSource, LayoutElement, OutcomeDataSourceDefinition,
    ProductionTriggerDefinition, RelationalConstraintKind, SemanticModule, TemplatePart, TextRange,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SymbolLocator {
    pub kind: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_id: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct SemanticReference {
    pub from: SymbolLocator,
    pub to: SymbolLocator,
    pub field: String,
    /// Source range of the referencing semantic record.
    pub span: TextRange,
}

impl SemanticReference {
    fn new(from: &SymbolLocator, to: &SymbolLocator, field: &str, span: TextRange) -> Self {
        Self {
            from: from.clone(),
            to: to.clone(),
            field: field.into(),
            span,
        }
    }
}

struct ReferenceCollector<'a> {
    module: &'a SemanticModule,
    canonical: BTreeMap<(String, String), SymbolLocator>,
    local: BTreeMap<(String, String, String), SymbolLocator>,
    references: Vec<SemanticReference>,
}

impl<'a> ReferenceCollector<'a> {
    fn new(module: &'a SemanticModule) -> Self {
        let mut collector = Self {
            module,
            canonical: BTreeMap::new(),
            local: BTreeMap::new(),
            references: Vec::new(),
        };
        collector.index_symbols();
        collector
    }

    fn canonical_locator(kind: &str, id: &CanonicalId) -> SymbolLocator {
        SymbolLocator {
            kind: kind.into(),
            id: id.to_string(),
            owner_id: None,
        }
    }

    fn local_locator(kind: &str, id: &str, owner_id: &CanonicalId) -> SymbolLocator {
        SymbolLocator {
            kind: kind.into(),
            id: id.into(),
            owner_id: Some(owner_id.to_string()),
        }
    }

    fn index_canonical(&mut self, kind: &str, id: &CanonicalId) {
        let key = (kind.into(), id.to_string());
        let previous = self
            .canonical
            .insert(key, Self::canonical_locator(kind, id));
        debug_assert!(
            previous.is_none(),
            "canonical symbol indexed twice: {kind} {id}"
        );
    }

    fn index_local(&mut self, kind: &str, id: &str, owner_id: &CanonicalId) {
        let locator = Self::local_locator(kind, id, owner_id);
        self.local
            .insert((kind.into(), owner_id.to_string(), id.into()), locator);
    }

    fn index_layout_elements(&mut self, screen_id: &CanonicalId, elements: &[LayoutElement]) {
        for element in elements {
            match element {
                LayoutElement::Header { id, children, .. }
                | LayoutElement::Section { id, children, .. } => {
                    if let Some(id) = id {
                        self.index_local("screen_layouts.elements", id, screen_id);
                    }
                    self.index_layout_elements(screen_id, children);
                }
                LayoutElement::Form { id, inputs, .. } => {
                    if let Some(id) = id {
                        self.index_local("screen_layouts.elements", id, screen_id);
                    }
                    self.index_layout_elements(screen_id, inputs);
                }
                LayoutElement::Heading { id, .. }
                | LayoutElement::Input { id, .. }
                | LayoutElement::List { id, .. }
                | LayoutElement::Placeholder { id, .. } => {
                    if let Some(id) = id {
                        self.index_local("screen_layouts.elements", id, screen_id);
                    }
                }
                LayoutElement::Button { id, .. } => {
                    self.index_local("screen_layouts.elements", id, screen_id);
                }
            }
        }
    }

    fn index_symbols(&mut self) {
        self.index_canonical("module", &self.module.id);
        for value in &self.module.enums {
            self.index_canonical("enums", &value.id);
            for nested in &value.variants {
                self.index_canonical("enums.variants", &nested.id);
            }
        }
        for value in &self.module.models {
            self.index_canonical("models", &value.id);
            for nested in &value.fields {
                self.index_canonical("models.fields", &nested.id);
            }
        }
        for value in &self.module.relations {
            self.index_canonical("relations", &value.id);
        }
        for value in &self.module.relational_constraints {
            self.index_canonical("relational_constraints", &value.id);
        }
        for value in &self.module.screens {
            self.index_canonical("screens", &value.id);
        }
        for value in &self.module.information_architecture {
            self.index_canonical("information_architecture", &value.id);
        }
        let layouts = self.module.screen_layouts.clone();
        for value in &layouts {
            self.index_layout_elements(&value.screen_id, &value.elements);
        }
        let paths = self.module.screen_paths.clone();
        for value in &paths {
            if let Some(id) = &value.id {
                self.index_local("screen_paths", id, &value.source_screen_id);
            }
            if let Some(handler) = &value.handler {
                self.index_local("screen_paths.handler", &handler.id, &value.source_screen_id);
            }
        }
        for value in &self.module.action_outcomes {
            self.index_canonical("action_outcomes", &value.id);
        }
        let lookups = self.module.lookup_results.clone();
        for value in &lookups {
            self.index_local("lookup_results", &value.id, &self.module.id);
        }
        for value in &self.module.workflows {
            self.index_canonical("workflows", &value.id);
        }
        for value in &self.module.constraints {
            self.index_canonical("constraints", &value.id);
        }
        for value in &self.module.roles {
            self.index_canonical("roles", &value.id);
        }
        for value in &self.module.actions {
            self.index_canonical("actions", &value.id);
            for nested in &value.inputs {
                self.index_canonical("actions.inputs", &nested.id);
            }
        }
        for value in &self.module.events {
            self.index_canonical("events", &value.id);
            for nested in &value.inputs {
                self.index_canonical("events.inputs", &nested.id);
            }
        }
        for value in &self.module.conditional_productions {
            self.index_canonical("conditional_productions", &value.id);
            for nested in &value.branches {
                self.index_canonical("conditional_productions.branches", &nested.id);
            }
            for nested in &value.field_producers {
                self.index_canonical("conditional_productions.field_producers", &nested.id);
            }
            for nested in &value.relation_producers {
                self.index_canonical("conditional_productions.relation_producers", &nested.id);
            }
        }
        for value in &self.module.policies {
            self.index_canonical("policies", &value.id);
        }
    }

    fn canonical(&self, kind: &str, id: &CanonicalId) -> Option<SymbolLocator> {
        self.canonical.get(&(kind.into(), id.to_string())).cloned()
    }

    fn local(&self, kind: &str, owner_id: &CanonicalId, id: &str) -> Option<SymbolLocator> {
        self.local
            .get(&(kind.into(), owner_id.to_string(), id.into()))
            .cloned()
    }

    fn push(
        &mut self,
        from: &SymbolLocator,
        target_kind: &str,
        target: &CanonicalId,
        field: &str,
        span: TextRange,
    ) {
        if let Some(to) = self.canonical(target_kind, target) {
            self.references
                .push(SemanticReference::new(from, &to, field, span));
        }
    }

    fn push_local(
        &mut self,
        from: &SymbolLocator,
        kind: &str,
        owner_id: &CanonicalId,
        target: &str,
        field: &str,
        span: TextRange,
    ) {
        if let Some(to) = self.local(kind, owner_id, target) {
            self.references
                .push(SemanticReference::new(from, &to, field, span));
        }
    }

    fn push_type(
        &mut self,
        from: &SymbolLocator,
        value: &CanonicalType,
        field: &str,
        span: TextRange,
    ) {
        match value {
            CanonicalType::Reference(id) => self.push(from, "models", id, field, span),
            CanonicalType::Enum(value) => {
                self.push(from, "enums", value.id(), field, span);
            }
            CanonicalType::List(value) | CanonicalType::Set(value) => {
                self.push_type(from, value, field, span);
            }
            CanonicalType::Map { key, value } => {
                self.push_type(from, key, field, span);
                self.push_type(from, value, field, span);
            }
            CanonicalType::Boolean
            | CanonicalType::Integer
            | CanonicalType::Decimal
            | CanonicalType::String
            | CanonicalType::Date
            | CanonicalType::Time
            | CanonicalType::DateTime
            | CanonicalType::Duration
            | CanonicalType::Latitude
            | CanonicalType::Longitude
            | CanonicalType::Money(_)
            | CanonicalType::Percentage
            | CanonicalType::Quantity(_)
            | CanonicalType::Coordinate
            | CanonicalType::LocalDateTime
            | CanonicalType::ZonedDateTime
            | CanonicalType::CalendarDuration
            | CanonicalType::Uuid
            | CanonicalType::Email
            | CanonicalType::Url
            | CanonicalType::PhoneNumber
            | CanonicalType::IpAddress
            | CanonicalType::Cidr
            | CanonicalType::CountryCode
            | CanonicalType::LanguageCode
            | CanonicalType::CurrencyCode
            | CanonicalType::Refinement(_) => {}
        }
    }

    fn collect_layout_elements(
        &mut self,
        screen: &SymbolLocator,
        screen_id: &CanonicalId,
        elements: &[LayoutElement],
    ) {
        for element in elements {
            let owner = match element {
                LayoutElement::Header { id, .. }
                | LayoutElement::Section { id, .. }
                | LayoutElement::Heading { id, .. }
                | LayoutElement::Form { id, .. }
                | LayoutElement::Input { id, .. }
                | LayoutElement::List { id, .. }
                | LayoutElement::Placeholder { id, .. } => id
                    .as_deref()
                    .and_then(|id| self.local("screen_layouts.elements", screen_id, id))
                    .unwrap_or_else(|| screen.clone()),
                LayoutElement::Button { id, .. } => self
                    .local("screen_layouts.elements", screen_id, id)
                    .unwrap_or_else(|| screen.clone()),
            };
            match element {
                LayoutElement::Header { children, .. }
                | LayoutElement::Section { children, .. } => {
                    self.collect_layout_elements(screen, screen_id, children);
                }
                LayoutElement::Form { inputs, .. } => {
                    self.collect_layout_elements(screen, screen_id, inputs);
                }
                LayoutElement::Input { field_id, span, .. } => {
                    self.push(&owner, "models.fields", field_id, "field_id", *span);
                }
                LayoutElement::List {
                    model_id,
                    field_ids,
                    span,
                    ..
                } => {
                    self.push(&owner, "models", model_id, "model_id", *span);
                    for field_id in field_ids {
                        self.push(&owner, "models.fields", field_id, "field_ids", *span);
                    }
                }
                LayoutElement::Button {
                    action_id, span, ..
                } => {
                    if let Some(action_id) = action_id {
                        self.push(&owner, "actions", action_id, "action_id", *span);
                    }
                }
                LayoutElement::Heading { .. } | LayoutElement::Placeholder { .. } => {}
            }
        }
    }

    fn collect(mut self) -> Vec<SemanticReference> {
        for value in &self.module.models {
            for field in &value.fields {
                let from = self
                    .canonical("models.fields", &field.id)
                    .expect("indexed field");
                self.push_type(&from, &field.value_type, "value_type", field.span);
            }
        }
        for value in &self.module.relations {
            let from = self
                .canonical("relations", &value.id)
                .expect("indexed relation");
            for target in &value.parameter_model_ids {
                self.push(&from, "models", target, "parameter_model_ids", value.span);
            }
        }
        for value in &self.module.relational_constraints {
            let from = self
                .canonical("relational_constraints", &value.id)
                .expect("indexed relational constraint");
            match &value.constraint {
                RelationalConstraintKind::NonEmpty { model_id } => {
                    self.push(&from, "models", model_id, "constraint.model_id", value.span);
                }
                RelationalConstraintKind::Required { relation_id }
                | RelationalConstraintKind::Unique { relation_id } => {
                    self.push(
                        &from,
                        "relations",
                        relation_id,
                        "constraint.relation_id",
                        value.span,
                    );
                }
                RelationalConstraintKind::Exclusive { relation_ids }
                | RelationalConstraintKind::Exhaustive { relation_ids }
                | RelationalConstraintKind::Coexistent { relation_ids } => {
                    for target in relation_ids {
                        self.push(
                            &from,
                            "relations",
                            target,
                            "constraint.relation_ids",
                            value.span,
                        );
                    }
                }
            }
        }
        for value in &self.module.screens {
            let from = self
                .canonical("screens", &value.id)
                .expect("indexed screen");
            for operation in &value.operations {
                self.push(
                    &from,
                    "models",
                    &operation.model_id,
                    "operations.model_id",
                    operation.span,
                );
                for target in &operation.field_ids {
                    self.push(
                        &from,
                        "models.fields",
                        target,
                        "operations.field_ids",
                        operation.span,
                    );
                }
            }
        }
        for value in &self.module.information_architecture {
            let from = self
                .canonical("information_architecture", &value.id)
                .expect("indexed category");
            if let Some(target) = &value.parent_id {
                self.push(
                    &from,
                    "information_architecture",
                    target,
                    "parent_id",
                    value.span,
                );
            }
        }
        for value in &self.module.screen_categories {
            let from = self
                .canonical("screens", &value.screen_id)
                .expect("indexed screen");
            self.push(&from, "screens", &value.screen_id, "screen_id", value.span);
            self.push(
                &from,
                "information_architecture",
                &value.category_id,
                "category_id",
                value.span,
            );
        }
        for value in &self.module.screen_layouts {
            let from = self
                .canonical("screens", &value.screen_id)
                .expect("indexed screen");
            self.push(&from, "screens", &value.screen_id, "screen_id", value.span);
            for target in &value.role_ids {
                self.push(&from, "roles", target, "role_ids", value.span);
            }
            for permission in &value.permissions {
                self.push(
                    &from,
                    "roles",
                    &permission.role_id,
                    "permissions.role_id",
                    permission.span,
                );
                self.push(
                    &from,
                    "actions",
                    &permission.action_id,
                    "permissions.action_id",
                    permission.span,
                );
                self.push(
                    &from,
                    "models",
                    &permission.model_id,
                    "permissions.model_id",
                    permission.span,
                );
                if let Some(target) = &permission.field_id {
                    self.push(
                        &from,
                        "models.fields",
                        target,
                        "permissions.field_id",
                        permission.span,
                    );
                }
            }
            self.collect_layout_elements(&from, &value.screen_id, &value.elements);
        }
        for value in &self.module.lookup_results {
            let from = self
                .local("lookup_results", &self.module.id, &value.id)
                .expect("indexed lookup");
            self.push(&from, "actions", &value.action_id, "action_id", value.span);
            self.push(
                &from,
                "actions.inputs",
                &value.input_id,
                "input_id",
                value.span,
            );
            self.push(&from, "models", &value.model_id, "model_id", value.span);
            for target in &value.field_ids {
                self.push(&from, "models.fields", target, "field_ids", value.span);
            }
        }
        for value in &self.module.action_outcomes {
            let from = self
                .canonical("action_outcomes", &value.id)
                .expect("indexed outcome");
            self.push(&from, "actions", &value.action_id, "action_id", value.span);
            for data in &value.provided_data {
                self.push(
                    &from,
                    "models",
                    &data.model_id,
                    "provided_data.model_id",
                    data.span,
                );
                self.push(
                    &from,
                    "models.fields",
                    &data.field_id,
                    "provided_data.field_id",
                    data.span,
                );
                for target in &data.prerequisite_field_ids {
                    self.push(
                        &from,
                        "models.fields",
                        target,
                        "provided_data.prerequisite_field_ids",
                        data.span,
                    );
                }
                match &data.source {
                    OutcomeDataSourceDefinition::Lookup { result_id } => self.push_local(
                        &from,
                        "lookup_results",
                        &self.module.id,
                        result_id,
                        "provided_data.source.result_id",
                        data.span,
                    ),
                    OutcomeDataSourceDefinition::Derivation {
                        target_field_id,
                        source_field_id,
                    } => {
                        self.push(
                            &from,
                            "models.fields",
                            target_field_id,
                            "provided_data.source.target_field_id",
                            data.span,
                        );
                        self.push(
                            &from,
                            "models.fields",
                            source_field_id,
                            "provided_data.source.source_field_id",
                            data.span,
                        );
                    }
                    OutcomeDataSourceDefinition::Producer { producer_id } => {
                        self.push(
                            &from,
                            "conditional_productions.field_producers",
                            producer_id,
                            "provided_data.source.producer_id",
                            data.span,
                        );
                    }
                }
            }
            if let Some(recovery) = &value.recovery {
                if let Some(target) = &recovery.screen_id {
                    self.push(
                        &from,
                        "screens",
                        target,
                        "recovery.screen_id",
                        recovery.span,
                    );
                    if let Some(element_id) = &recovery.element_id {
                        self.push_local(
                            &from,
                            "screen_layouts.elements",
                            target,
                            element_id,
                            "recovery.element_id",
                            recovery.span,
                        );
                    }
                }
                if let Some(target) = &recovery.action_id {
                    self.push(
                        &from,
                        "actions",
                        target,
                        "recovery.action_id",
                        recovery.span,
                    );
                }
                if let Some(path_id) = &recovery.path_id {
                    for path in &self.module.screen_paths {
                        if path.id.as_deref() == Some(path_id) {
                            self.push_local(
                                &from,
                                "screen_paths",
                                &path.source_screen_id,
                                path_id,
                                "recovery.path_id",
                                recovery.span,
                            );
                            break;
                        }
                    }
                }
            }
        }
        for value in &self.module.screen_paths {
            let from = value
                .id
                .as_deref()
                .and_then(|id| self.local("screen_paths", &value.source_screen_id, id))
                .or_else(|| self.canonical("screens", &value.source_screen_id))
                .expect("path owner");
            self.push(
                &from,
                "screens",
                &value.source_screen_id,
                "source_screen_id",
                value.span,
            );
            self.push_local(
                &from,
                "screen_layouts.elements",
                &value.source_screen_id,
                &value.source_element_id,
                "source_element_id",
                value.span,
            );
            if let Some(target) = &value.target_screen_id {
                self.push(&from, "screens", target, "target_screen_id", value.span);
            }
            if let Some(target) = &value.outcome_id {
                self.push(&from, "action_outcomes", target, "outcome_id", value.span);
            }
        }
        for value in &self.module.workflows {
            let from = self
                .canonical("workflows", &value.id)
                .expect("indexed workflow");
            self.push(
                &from,
                "screens",
                &value.start_screen_id,
                "start_screen_id",
                value.span,
            );
            for data in &value.initial_data {
                self.push(
                    &from,
                    "models",
                    &data.model_id,
                    "initial_data.model_id",
                    data.span,
                );
                self.push(
                    &from,
                    "models.fields",
                    &data.field_id,
                    "initial_data.field_id",
                    data.span,
                );
            }
            for acquisition in &value.acquisitions {
                self.push(
                    &from,
                    "screens",
                    &acquisition.source_screen_id,
                    "acquisitions.source_screen_id",
                    acquisition.span,
                );
                self.push_local(
                    &from,
                    "screen_layouts.elements",
                    &acquisition.source_screen_id,
                    &acquisition.source_element_id,
                    "acquisitions.source_element_id",
                    acquisition.span,
                );
                for data in &acquisition.data {
                    self.push(
                        &from,
                        "models",
                        &data.model_id,
                        "acquisitions.data.model_id",
                        data.span,
                    );
                    self.push(
                        &from,
                        "models.fields",
                        &data.field_id,
                        "acquisitions.data.field_id",
                        data.span,
                    );
                }
            }
            for completion in &value.completions {
                self.push(
                    &from,
                    "screens",
                    &completion.screen_id,
                    "completions.screen_id",
                    completion.span,
                );
                for data in &completion.required_data {
                    self.push(
                        &from,
                        "models",
                        &data.model_id,
                        "completions.required_data.model_id",
                        data.span,
                    );
                    self.push(
                        &from,
                        "models.fields",
                        &data.field_id,
                        "completions.required_data.field_id",
                        data.span,
                    );
                }
            }
        }
        for value in &self.module.action_data_mutations {
            let from = self
                .canonical("actions", &value.action_id)
                .expect("indexed action");
            self.push(&from, "actions", &value.action_id, "action_id", value.span);
            self.push(&from, "models", &value.model_id, "model_id", value.span);
        }
        for value in &self.module.derivations {
            let from = self
                .canonical("models.fields", &value.target_field_id)
                .expect("indexed field");
            self.push(
                &from,
                "models.fields",
                &value.target_field_id,
                "target_field_id",
                value.span,
            );
            let crate::DerivationExpression::Sum { source_field_id } = &value.expression;
            self.push(
                &from,
                "models.fields",
                source_field_id,
                "expression.source_field_id",
                value.span,
            );
            for target in &value.recalculate_when_changed_field_ids {
                self.push(
                    &from,
                    "models.fields",
                    target,
                    "recalculate_when_changed_field_ids",
                    value.span,
                );
            }
        }
        for value in &self.module.recalculations {
            let from = self
                .canonical("models.fields", &value.target_field_id)
                .expect("indexed field");
            self.push(
                &from,
                "models.fields",
                &value.source_field_id,
                "source_field_id",
                value.span,
            );
            self.push(
                &from,
                "models.fields",
                &value.target_field_id,
                "target_field_id",
                value.span,
            );
        }
        for value in &self.module.field_intents {
            let from = self
                .canonical("models.fields", &value.field_id)
                .expect("indexed field");
            self.push(
                &from,
                "models.fields",
                &value.field_id,
                "field_id",
                value.span,
            );
        }
        for value in &self.module.constraints {
            let from = self
                .canonical("constraints", &value.id)
                .expect("indexed constraint");
            self.push(&from, "models", &value.model_id, "model_id", value.span);
            if let ConstraintOperand::Field(target) = &value.left {
                self.push(&from, "models.fields", target, "left", value.span);
            }
            if let ConstraintOperand::Field(target) = &value.right {
                self.push(&from, "models.fields", target, "right", value.span);
            }
        }
        for value in &self.module.actions {
            for input in &value.inputs {
                let from = self
                    .canonical("actions.inputs", &input.id)
                    .expect("indexed action input");
                match &input.kind {
                    ActionInputKind::ExistingModel { model_id } => {
                        self.push(&from, "models", model_id, "kind.model_id", input.span);
                    }
                    ActionInputKind::Value { value_type } => {
                        self.push_type(&from, value_type, "kind.value_type", input.span);
                    }
                }
            }
        }
        for value in &self.module.events {
            for input in &value.inputs {
                let from = self
                    .canonical("events.inputs", &input.id)
                    .expect("indexed event input");
                match &input.kind {
                    EventInputKind::ExistingModel { model_id } => {
                        self.push(&from, "models", model_id, "kind.model_id", input.span);
                    }
                    EventInputKind::Value { value_type } => {
                        self.push_type(&from, value_type, "kind.value_type", input.span);
                    }
                }
            }
        }
        for value in &self.module.conditional_productions {
            let from = self
                .canonical("conditional_productions", &value.id)
                .expect("indexed production");
            if let Some(target) = &value.action_id {
                self.push(&from, "actions", target, "action_id", value.span);
            }
            let input_kind = match &value.trigger {
                ProductionTriggerDefinition::Action(target) => {
                    self.push(&from, "actions", target, "trigger.action", value.span);
                    "actions.inputs"
                }
                ProductionTriggerDefinition::Event(target) => {
                    self.push(&from, "events", target, "trigger.event", value.span);
                    "events.inputs"
                }
            };
            self.push(
                &from,
                "models",
                &value.output_model_id,
                "output_model_id",
                value.span,
            );
            self.push(
                &from,
                input_kind,
                &value.decision_input_id,
                "decision_input_id",
                value.span,
            );
            for branch in &value.branches {
                let branch_from = self
                    .canonical("conditional_productions.branches", &branch.id)
                    .expect("indexed branch");
                self.push(
                    &branch_from,
                    "enums.variants",
                    &branch.variant_id,
                    "variant_id",
                    branch.span,
                );
            }
            for producer in &value.field_producers {
                let producer_from = self
                    .canonical("conditional_productions.field_producers", &producer.id)
                    .expect("indexed field producer");
                self.push(
                    &producer_from,
                    "models.fields",
                    &producer.output_field_id,
                    "output_field_id",
                    producer.span,
                );
                match &producer.source {
                    FieldProducerSource::ActionInput { input_id }
                    | FieldProducerSource::EventInput { input_id } => {
                        self.push(
                            &producer_from,
                            input_kind,
                            input_id,
                            "source.input_id",
                            producer.span,
                        );
                    }
                    FieldProducerSource::InputField { input_id, field_id }
                    | FieldProducerSource::EventInputField { input_id, field_id } => {
                        self.push(
                            &producer_from,
                            input_kind,
                            input_id,
                            "source.input_id",
                            producer.span,
                        );
                        self.push(
                            &producer_from,
                            "models.fields",
                            field_id,
                            "source.field_id",
                            producer.span,
                        );
                    }
                    FieldProducerSource::Template { parts } => {
                        for part in parts {
                            if let TemplatePart::OutputField { field_id } = part {
                                self.push(
                                    &producer_from,
                                    "models.fields",
                                    field_id,
                                    "source.parts.field_id",
                                    producer.span,
                                );
                            }
                        }
                    }
                    FieldProducerSource::Constant { .. } => {}
                }
                if let Some(FieldProducerCondition::EnumVariant {
                    input_id,
                    variant_id,
                }) = &producer.condition
                {
                    self.push(
                        &producer_from,
                        input_kind,
                        input_id,
                        "condition.input_id",
                        producer.span,
                    );
                    self.push(
                        &producer_from,
                        "enums.variants",
                        variant_id,
                        "condition.variant_id",
                        producer.span,
                    );
                }
            }
            for target in &value.field_evaluation_order {
                self.push(
                    &from,
                    "models.fields",
                    target,
                    "field_evaluation_order",
                    value.span,
                );
            }
            for slot in &value.relation_slots {
                self.push(
                    &from,
                    "relations",
                    &slot.relation_id,
                    "relation_slots.relation_id",
                    slot.span,
                );
                self.push(
                    &from,
                    "models",
                    &slot.output_model_id,
                    "relation_slots.output_model_id",
                    slot.span,
                );
                self.push(
                    &from,
                    "models",
                    &slot.endpoint_model_id,
                    "relation_slots.endpoint_model_id",
                    slot.span,
                );
            }
            for producer in &value.relation_producers {
                let producer_from = self
                    .canonical("conditional_productions.relation_producers", &producer.id)
                    .expect("indexed relation producer");
                self.push(
                    &producer_from,
                    "relations",
                    &producer.relation_id,
                    "relation_id",
                    producer.span,
                );
                self.push(
                    &producer_from,
                    input_kind,
                    &producer.input_id,
                    "input_id",
                    producer.span,
                );
            }
        }
        for value in &self.module.policies {
            let from = self
                .canonical("policies", &value.id)
                .expect("indexed policy");
            self.push(&from, "roles", &value.role_id, "role_id", value.span);
            self.push(&from, "models", &value.model_id, "model_id", value.span);
            self.push(
                &from,
                "models.fields",
                &value.field_id,
                "field_id",
                value.span,
            );
            self.push(&from, "actions", &value.action_id, "action_id", value.span);
        }
        self.references.sort();
        self.references.dedup();
        self.references
    }
}

pub fn semantic_references(module: &SemanticModule) -> Vec<SemanticReference> {
    ReferenceCollector::new(module).collect()
}
