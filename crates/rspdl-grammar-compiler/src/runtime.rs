use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Expr {
    Literal(String),
    RuleRef(String),
    Matcher {
        name: String,
        arguments: Vec<String>,
    },
    Sequence(Vec<Expr>),
    Choice(Vec<Expr>),
    Optional(Box<Expr>),
    Repeat {
        expression: Box<Expr>,
        minimum: usize,
    },
    Capture {
        name: String,
        expression: Box<Expr>,
    },
}

impl Expr {
    pub fn literal(value: impl Into<String>) -> Self {
        Self::Literal(value.into())
    }

    pub fn rule_ref(name: impl Into<String>) -> Self {
        Self::RuleRef(name.into())
    }

    pub fn matcher(name: impl Into<String>, arguments: Vec<String>) -> Self {
        Self::Matcher {
            name: name.into(),
            arguments,
        }
    }

    pub fn sequence(expressions: Vec<Self>) -> Self {
        Self::Sequence(expressions)
    }

    pub fn choice(expressions: Vec<Self>) -> Self {
        Self::Choice(expressions)
    }

    pub fn optional(expression: Self) -> Self {
        Self::Optional(Box::new(expression))
    }

    pub fn repeat(expression: Self, minimum: usize) -> Self {
        Self::Repeat {
            expression: Box::new(expression),
            minimum,
        }
    }

    pub fn capture(name: impl Into<String>, expression: Self) -> Self {
        Self::Capture {
            name: name.into(),
            expression: Box::new(expression),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    pub name: String,
    pub expression: Expr,
}

impl Rule {
    pub fn new(name: impl Into<String>, expression: Expr) -> Self {
        Self {
            name: name.into(),
            expression,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Capture {
    pub value: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalMatch {
    pub end: usize,
    pub value: String,
    pub start_offset: usize,
    pub end_offset: usize,
}

impl TerminalMatch {
    pub fn new(
        end: usize,
        value: impl Into<String>,
        start_offset: usize,
        end_offset: usize,
    ) -> Self {
        Self {
            end,
            value: value.into(),
            start_offset,
            end_offset,
        }
    }
}

pub trait InputAdapter<T> {
    fn match_literal(&self, tokens: &[T], position: usize, literal: &str) -> Option<TerminalMatch>;

    fn match_contextual(
        &self,
        tokens: &[T],
        position: usize,
        matcher: &str,
        arguments: &[String],
    ) -> Vec<TerminalMatch>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseMatch {
    pub captures: BTreeMap<String, Vec<Capture>>,
}

impl ParseMatch {
    pub fn capture(&self, name: &str) -> Option<&Capture> {
        self.captures.get(name).and_then(|values| values.first())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseFailure {
    pub position: usize,
    pub expected: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    UnknownEntry { name: String },
    NoMatch(ParseFailure),
    Ambiguous { alternatives: usize },
    LimitExceeded { limit: ParseLimit, maximum: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseLimit {
    Outcomes,
    RuleDepth,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParseLimits {
    pub max_outcomes: usize,
    pub max_rule_depth: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_outcomes: 1_000_000,
            max_rule_depth: 256,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GrammarDefinitionError {
    DuplicateRule { name: String },
    UnknownPublicRule { name: String },
    UndefinedRule { rule: String, reference: String },
    NullableCapture { rule: String },
}

impl std::fmt::Display for GrammarDefinitionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateRule { name } => write!(formatter, "rule {name:?} is defined twice"),
            Self::UnknownPublicRule { name } => {
                write!(formatter, "public rule {name:?} is not defined")
            }
            Self::UndefinedRule { rule, reference } => {
                write!(
                    formatter,
                    "rule {rule:?} references undefined rule {reference:?}"
                )
            }
            Self::NullableCapture { rule } => {
                write!(formatter, "rule {rule:?} contains a nullable capture")
            }
        }
    }
}

impl std::error::Error for GrammarDefinitionError {}

#[derive(Clone, Debug)]
pub struct Grammar {
    public_rules: BTreeSet<String>,
    rules: BTreeMap<String, Expr>,
}

impl Grammar {
    #[doc(hidden)]
    pub fn from_generated_parts(
        public_rules: Vec<String>,
        rules: Vec<Rule>,
    ) -> Result<Self, GrammarDefinitionError> {
        let mut definitions = BTreeMap::new();
        for rule in rules {
            if definitions
                .insert(rule.name.clone(), rule.expression)
                .is_some()
            {
                return Err(GrammarDefinitionError::DuplicateRule { name: rule.name });
            }
        }
        let public_rules = public_rules.into_iter().collect::<BTreeSet<_>>();
        for name in &public_rules {
            if !definitions.contains_key(name) {
                return Err(GrammarDefinitionError::UnknownPublicRule { name: name.clone() });
            }
        }
        for (rule, expression) in &definitions {
            validate_rule_references(rule, expression, &definitions)?;
        }
        let nullable = definition_nullable_rules(&definitions);
        for (rule, expression) in &definitions {
            if has_nullable_capture(expression, &nullable) {
                return Err(GrammarDefinitionError::NullableCapture { rule: rule.clone() });
            }
        }
        Ok(Self {
            public_rules,
            rules: definitions,
        })
    }

    pub fn parse<T>(
        &self,
        entry: &str,
        tokens: &[T],
        adapter: &impl InputAdapter<T>,
    ) -> Result<ParseMatch, ParseError> {
        self.parse_with_limits(entry, tokens, adapter, ParseLimits::default())
    }

    pub fn parse_with_limits<T>(
        &self,
        entry: &str,
        tokens: &[T],
        adapter: &impl InputAdapter<T>,
        limits: ParseLimits,
    ) -> Result<ParseMatch, ParseError> {
        if !self.public_rules.contains(entry) {
            return Err(ParseError::UnknownEntry {
                name: entry.to_owned(),
            });
        }
        let expression = self
            .rules
            .get(entry)
            .ok_or_else(|| ParseError::UnknownEntry {
                name: entry.to_owned(),
            })?;
        let mut failure = FailureAccumulator::default();
        let mut context = EvaluationContext::new(limits);
        let mut session = EvaluationSession {
            failure: &mut failure,
            context: &mut context,
            rule_depth: 0,
        };
        let initial = State {
            position: 0,
            captures: BTreeMap::new(),
        };
        let outcomes = self.evaluate(expression, initial, tokens, adapter, &mut session)?;
        if let Some(position) = outcomes.iter().map(|outcome| outcome.position).max() {
            failure.record(position, "<end of input>");
        }
        let complete = outcomes
            .into_iter()
            .filter(|outcome| outcome.position == tokens.len())
            .collect::<Vec<_>>();
        match complete.len() {
            0 => Err(ParseError::NoMatch(failure.finish())),
            1 => Ok(ParseMatch {
                captures: complete
                    .into_iter()
                    .next()
                    .expect("one complete outcome")
                    .captures,
            }),
            alternatives => Err(ParseError::Ambiguous { alternatives }),
        }
    }

    pub fn public_rules(&self) -> impl Iterator<Item = &str> {
        self.public_rules.iter().map(String::as_str)
    }

    fn evaluate<T>(
        &self,
        expression: &Expr,
        state: State,
        tokens: &[T],
        adapter: &impl InputAdapter<T>,
        session: &mut EvaluationSession<'_>,
    ) -> Result<Vec<Outcome>, ParseError> {
        let outcomes = match expression {
            Expr::Literal(literal) => {
                let position = state.position;
                match adapter.match_literal(tokens, position, literal) {
                    Some(value) if valid_terminal_match(&value, position, tokens.len()) => {
                        vec![Outcome::terminal(state.captures, value)]
                    }
                    _ => {
                        session.failure.record(position, format!("{literal:?}"));
                        Vec::new()
                    }
                }
            }
            Expr::Matcher { name, arguments } => {
                let position = state.position;
                let values = adapter.match_contextual(tokens, position, name, arguments);
                let outcomes = values
                    .into_iter()
                    .filter(|value| valid_terminal_match(value, position, tokens.len()))
                    .map(|value| Outcome::terminal(state.captures.clone(), value))
                    .collect::<Vec<_>>();
                if outcomes.is_empty() {
                    session.failure.record(position, format!("@{name}"));
                }
                outcomes
            }
            Expr::RuleRef(name) => {
                if session.rule_depth >= session.context.limits.max_rule_depth {
                    return Err(ParseError::LimitExceeded {
                        limit: ParseLimit::RuleDepth,
                        maximum: session.context.limits.max_rule_depth,
                    });
                }
                let key = MemoKey {
                    name: name.clone(),
                    position: state.position,
                    captures: state.captures.clone(),
                    rule_depth: session.rule_depth,
                };
                if let Some(cached) = session.context.memo.get(&key).cloned() {
                    session.failure.merge(&cached.failure);
                    return session.context.retain(cached.outcomes);
                }
                let expression = self
                    .rules
                    .get(name)
                    .ok_or_else(|| ParseError::UnknownEntry { name: name.clone() })?;
                let mut local_failure = FailureAccumulator::default();
                let mut nested_session = EvaluationSession {
                    failure: &mut local_failure,
                    context: session.context,
                    rule_depth: session.rule_depth + 1,
                };
                let outcomes =
                    self.evaluate(expression, state, tokens, adapter, &mut nested_session)?;
                session.failure.merge(&local_failure);
                session.context.memo.insert(
                    key,
                    CachedRule {
                        outcomes: outcomes.clone(),
                        failure: local_failure,
                    },
                );
                outcomes
            }
            Expr::Sequence(expressions) => {
                let mut outcomes = vec![Outcome::empty(state)];
                for expression in expressions {
                    let mut next = Vec::new();
                    for outcome in outcomes {
                        let children = self.evaluate(
                            expression,
                            outcome.as_state(),
                            tokens,
                            adapter,
                            session,
                        )?;
                        for child in children {
                            next.push(outcome.clone().append(child));
                        }
                    }
                    if next.is_empty() {
                        return Ok(Vec::new());
                    }
                    session.context.record(next.len())?;
                    outcomes = next;
                }
                outcomes
            }
            Expr::Choice(expressions) => {
                let mut outcomes = Vec::new();
                for expression in expressions {
                    outcomes.extend(self.evaluate(
                        expression,
                        state.clone(),
                        tokens,
                        adapter,
                        session,
                    )?);
                }
                outcomes
            }
            Expr::Optional(expression) => {
                let mut outcomes = vec![Outcome::empty(state.clone())];
                outcomes.extend(self.evaluate(expression, state, tokens, adapter, session)?);
                outcomes
            }
            Expr::Repeat {
                expression,
                minimum,
            } => self.evaluate_repeat(expression, *minimum, state, tokens, adapter, session)?,
            Expr::Capture { name, expression } => self
                .evaluate(expression, state, tokens, adapter, session)?
                .into_iter()
                .map(|mut outcome| {
                    let captured = combine_values(&outcome.values)
                        .expect("validated capture expressions are not nullable");
                    outcome
                        .captures
                        .entry(name.clone())
                        .or_default()
                        .push(captured.clone());
                    outcome.values = vec![captured];
                    outcome
                })
                .collect(),
        };
        session.context.retain(outcomes)
    }

    fn evaluate_repeat<T>(
        &self,
        expression: &Expr,
        minimum: usize,
        state: State,
        tokens: &[T],
        adapter: &impl InputAdapter<T>,
        session: &mut EvaluationSession<'_>,
    ) -> Result<Vec<Outcome>, ParseError> {
        let initial = Outcome::empty(state);
        let mut accepted = if minimum == 0 {
            vec![initial.clone()]
        } else {
            Vec::new()
        };
        let mut frontier = vec![(0usize, initial)];
        while !frontier.is_empty() {
            let mut next = Vec::new();
            for (count, outcome) in frontier {
                for child in
                    self.evaluate(expression, outcome.as_state(), tokens, adapter, session)?
                {
                    if child.position <= outcome.position {
                        continue;
                    }
                    let combined = outcome.clone().append(child);
                    let next_count = count + 1;
                    if next_count >= minimum {
                        session.context.record(1)?;
                        accepted.push(combined.clone());
                    }
                    session.context.record(1)?;
                    next.push((next_count, combined));
                }
            }
            frontier = next;
        }
        Ok(accepted)
    }
}

fn validate_rule_references(
    rule: &str,
    expression: &Expr,
    rules: &BTreeMap<String, Expr>,
) -> Result<(), GrammarDefinitionError> {
    match expression {
        Expr::RuleRef(reference) if !rules.contains_key(reference) => {
            Err(GrammarDefinitionError::UndefinedRule {
                rule: rule.to_owned(),
                reference: reference.clone(),
            })
        }
        Expr::Sequence(expressions) | Expr::Choice(expressions) => {
            for expression in expressions {
                validate_rule_references(rule, expression, rules)?;
            }
            Ok(())
        }
        Expr::Optional(expression)
        | Expr::Repeat { expression, .. }
        | Expr::Capture { expression, .. } => validate_rule_references(rule, expression, rules),
        Expr::Literal(_) | Expr::RuleRef(_) | Expr::Matcher { .. } => Ok(()),
    }
}

fn definition_nullable_rules(rules: &BTreeMap<String, Expr>) -> BTreeMap<String, bool> {
    let mut nullable = rules
        .keys()
        .map(|name| (name.clone(), false))
        .collect::<BTreeMap<_, _>>();
    loop {
        let mut changed = false;
        for (name, expression) in rules {
            if !nullable[name] && definition_is_nullable(expression, &nullable) {
                nullable.insert(name.clone(), true);
                changed = true;
            }
        }
        if !changed {
            return nullable;
        }
    }
}

fn definition_is_nullable(expression: &Expr, nullable: &BTreeMap<String, bool>) -> bool {
    match expression {
        Expr::Literal(_) | Expr::Matcher { .. } => false,
        Expr::RuleRef(name) => nullable.get(name).copied().unwrap_or(false),
        Expr::Sequence(expressions) => expressions
            .iter()
            .all(|expression| definition_is_nullable(expression, nullable)),
        Expr::Choice(expressions) => expressions
            .iter()
            .any(|expression| definition_is_nullable(expression, nullable)),
        Expr::Optional(_) => true,
        Expr::Repeat {
            expression,
            minimum,
        } => *minimum == 0 || definition_is_nullable(expression, nullable),
        Expr::Capture { expression, .. } => definition_is_nullable(expression, nullable),
    }
}

fn has_nullable_capture(expression: &Expr, nullable: &BTreeMap<String, bool>) -> bool {
    match expression {
        Expr::Capture { expression, .. } if definition_is_nullable(expression, nullable) => true,
        Expr::Sequence(expressions) | Expr::Choice(expressions) => expressions
            .iter()
            .any(|expression| has_nullable_capture(expression, nullable)),
        Expr::Optional(expression)
        | Expr::Repeat { expression, .. }
        | Expr::Capture { expression, .. } => has_nullable_capture(expression, nullable),
        Expr::Literal(_) | Expr::RuleRef(_) | Expr::Matcher { .. } => false,
    }
}

fn valid_terminal_match(value: &TerminalMatch, position: usize, token_count: usize) -> bool {
    value.end > position && value.end <= token_count && value.start_offset <= value.end_offset
}

fn combine_values(values: &[Capture]) -> Option<Capture> {
    let first = values.first()?;
    let last = values.last()?;
    Some(Capture {
        value: values
            .iter()
            .map(|value| value.value.as_str())
            .collect::<Vec<_>>()
            .join(" "),
        start: first.start,
        end: last.end,
    })
}

#[derive(Clone, Debug)]
struct State {
    position: usize,
    captures: BTreeMap<String, Vec<Capture>>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct MemoKey {
    name: String,
    position: usize,
    captures: BTreeMap<String, Vec<Capture>>,
    rule_depth: usize,
}

#[derive(Clone, Debug)]
struct CachedRule {
    outcomes: Vec<Outcome>,
    failure: FailureAccumulator,
}

struct EvaluationContext {
    limits: ParseLimits,
    generated_outcomes: usize,
    memo: BTreeMap<MemoKey, CachedRule>,
}

struct EvaluationSession<'a> {
    failure: &'a mut FailureAccumulator,
    context: &'a mut EvaluationContext,
    rule_depth: usize,
}

impl EvaluationContext {
    fn new(limits: ParseLimits) -> Self {
        Self {
            limits,
            generated_outcomes: 0,
            memo: BTreeMap::new(),
        }
    }

    fn record(&mut self, count: usize) -> Result<(), ParseError> {
        self.generated_outcomes = self.generated_outcomes.saturating_add(count);
        if self.generated_outcomes > self.limits.max_outcomes {
            return Err(ParseError::LimitExceeded {
                limit: ParseLimit::Outcomes,
                maximum: self.limits.max_outcomes,
            });
        }
        Ok(())
    }

    fn retain(&mut self, outcomes: Vec<Outcome>) -> Result<Vec<Outcome>, ParseError> {
        self.record(outcomes.len())?;
        Ok(outcomes)
    }
}

#[derive(Clone, Debug)]
struct Outcome {
    position: usize,
    captures: BTreeMap<String, Vec<Capture>>,
    values: Vec<Capture>,
}

impl Outcome {
    fn empty(state: State) -> Self {
        Self {
            position: state.position,
            captures: state.captures,
            values: Vec::new(),
        }
    }

    fn terminal(captures: BTreeMap<String, Vec<Capture>>, value: TerminalMatch) -> Self {
        Self {
            position: value.end,
            captures,
            values: vec![Capture {
                value: value.value,
                start: value.start_offset,
                end: value.end_offset,
            }],
        }
    }

    fn as_state(&self) -> State {
        State {
            position: self.position,
            captures: self.captures.clone(),
        }
    }

    fn append(mut self, child: Self) -> Self {
        self.position = child.position;
        self.captures = child.captures;
        self.values.extend(child.values);
        self
    }
}

#[derive(Clone, Debug, Default)]
struct FailureAccumulator {
    position: usize,
    expected: BTreeSet<String>,
    initialized: bool,
}

impl FailureAccumulator {
    fn record(&mut self, position: usize, expected: impl Into<String>) {
        if !self.initialized || position > self.position {
            self.position = position;
            self.expected.clear();
            self.initialized = true;
        }
        if position == self.position {
            self.expected.insert(expected.into());
        }
    }

    fn finish(self) -> ParseFailure {
        ParseFailure {
            position: self.position,
            expected: self.expected,
        }
    }

    fn merge(&mut self, other: &Self) {
        for expected in &other.expected {
            self.record(other.position, expected.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Words;

    impl InputAdapter<&str> for Words {
        fn match_literal(
            &self,
            tokens: &[&str],
            position: usize,
            literal: &str,
        ) -> Option<TerminalMatch> {
            (tokens.get(position).copied() == Some(literal))
                .then(|| TerminalMatch::new(position + 1, literal, position, position + 1))
        }

        fn match_contextual(
            &self,
            tokens: &[&str],
            position: usize,
            matcher: &str,
            arguments: &[String],
        ) -> Vec<TerminalMatch> {
            if matcher != "suffix" {
                return Vec::new();
            }
            tokens
                .get(position)
                .into_iter()
                .flat_map(|token| {
                    arguments.iter().filter_map(move |suffix| {
                        token.strip_suffix(suffix).and_then(|base| {
                            (!base.is_empty()).then(|| {
                                TerminalMatch::new(position + 1, base, position, position + 1)
                            })
                        })
                    })
                })
                .collect()
        }
    }

    #[test]
    fn captures_literal_and_contextual_matches() {
        let grammar = Grammar::from_generated_parts(
            vec!["greeting".into()],
            vec![Rule::new(
                "greeting",
                Expr::sequence(vec![
                    Expr::capture(
                        "name",
                        Expr::matcher("suffix", vec!["는".into(), "은".into()]),
                    ),
                    Expr::literal("온다"),
                ]),
            )],
        )
        .unwrap();
        let parsed = grammar
            .parse("greeting", &["민수는", "온다"], &Words)
            .unwrap();
        assert_eq!(parsed.capture("name").unwrap().value, "민수");
    }

    #[test]
    fn reports_the_farthest_expectation() {
        let grammar = Grammar::from_generated_parts(
            vec!["entry".into()],
            vec![Rule::new(
                "entry",
                Expr::choice(vec![
                    Expr::sequence(vec![Expr::literal("a"), Expr::literal("b")]),
                    Expr::sequence(vec![Expr::literal("a"), Expr::literal("c")]),
                ]),
            )],
        )
        .unwrap();
        let error = grammar.parse("entry", &["a", "d"], &Words).unwrap_err();
        let ParseError::NoMatch(failure) = error else {
            panic!("expected a failure");
        };
        assert_eq!(failure.position, 1);
        assert_eq!(
            failure.expected,
            BTreeSet::from(["\"b\"".into(), "\"c\"".into()])
        );
    }

    #[test]
    fn rejects_ambiguous_complete_parses() {
        let grammar = Grammar::from_generated_parts(
            vec!["entry".into()],
            vec![Rule::new(
                "entry",
                Expr::choice(vec![Expr::literal("same"), Expr::literal("same")]),
            )],
        )
        .unwrap();
        assert_eq!(
            grammar.parse("entry", &["same"], &Words),
            Err(ParseError::Ambiguous { alternatives: 2 })
        );
    }

    #[test]
    fn rejects_malformed_generated_grammar_without_parse_panics() {
        assert_eq!(
            Grammar::from_generated_parts(vec!["missing".into()], Vec::new()).unwrap_err(),
            GrammarDefinitionError::UnknownPublicRule {
                name: "missing".into()
            }
        );
        assert_eq!(
            Grammar::from_generated_parts(
                vec!["entry".into()],
                vec![Rule::new("entry", Expr::rule_ref("missing"))],
            )
            .unwrap_err(),
            GrammarDefinitionError::UndefinedRule {
                rule: "entry".into(),
                reference: "missing".into(),
            }
        );
        assert_eq!(
            Grammar::from_generated_parts(
                vec!["entry".into()],
                vec![Rule::new(
                    "entry",
                    Expr::capture("value", Expr::optional(Expr::literal("x"))),
                )],
            )
            .unwrap_err(),
            GrammarDefinitionError::NullableCapture {
                rule: "entry".into()
            }
        );
    }

    #[test]
    fn matcher_repetition_and_repeated_parses_are_deterministic() {
        let grammar = Grammar::from_generated_parts(
            vec!["entry".into()],
            vec![Rule::new(
                "entry",
                Expr::repeat(
                    Expr::capture(
                        "name",
                        Expr::matcher("suffix", vec!["는".into(), "수는".into()]),
                    ),
                    1,
                ),
            )],
        )
        .unwrap();
        let expected = Err(ParseError::Ambiguous { alternatives: 2 });
        for _ in 0..10 {
            assert_eq!(grammar.parse("entry", &["민수는"], &Words), expected);
        }
    }

    #[test]
    fn configurable_limits_bound_outcomes_and_rule_depth() {
        let ambiguous = Grammar::from_generated_parts(
            vec!["entry".into()],
            vec![Rule::new(
                "entry",
                Expr::choice(vec![
                    Expr::literal("x"),
                    Expr::literal("x"),
                    Expr::literal("x"),
                ]),
            )],
        )
        .unwrap();
        assert_eq!(
            ambiguous.parse_with_limits(
                "entry",
                &["x"],
                &Words,
                ParseLimits {
                    max_outcomes: 2,
                    ..ParseLimits::default()
                },
            ),
            Err(ParseError::LimitExceeded {
                limit: ParseLimit::Outcomes,
                maximum: 2,
            })
        );

        let recursive = Grammar::from_generated_parts(
            vec!["entry".into()],
            vec![Rule::new(
                "entry",
                Expr::choice(vec![
                    Expr::literal("done"),
                    Expr::sequence(vec![Expr::literal("x"), Expr::rule_ref("entry")]),
                ]),
            )],
        )
        .unwrap();
        assert_eq!(
            recursive.parse_with_limits(
                "entry",
                &["x", "x", "done"],
                &Words,
                ParseLimits {
                    max_rule_depth: 1,
                    ..ParseLimits::default()
                },
            ),
            Err(ParseError::LimitExceeded {
                limit: ParseLimit::RuleDepth,
                maximum: 1,
            })
        );
    }
}
