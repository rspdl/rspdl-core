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

#[derive(Clone, Debug, Eq, PartialEq)]
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
}

#[derive(Clone, Debug)]
pub struct Grammar {
    public_rules: BTreeSet<String>,
    rules: BTreeMap<String, Expr>,
}

impl Grammar {
    #[doc(hidden)]
    pub fn from_generated_parts(public_rules: Vec<String>, rules: Vec<Rule>) -> Self {
        Self {
            public_rules: public_rules.into_iter().collect(),
            rules: rules
                .into_iter()
                .map(|rule| (rule.name, rule.expression))
                .collect(),
        }
    }

    pub fn public_rules(&self) -> impl Iterator<Item = &str> {
        self.public_rules.iter().map(String::as_str)
    }

    pub fn parse<T>(
        &self,
        entry: &str,
        tokens: &[T],
        adapter: &impl InputAdapter<T>,
    ) -> Result<ParseMatch, ParseError> {
        if !self.public_rules.contains(entry) {
            return Err(ParseError::UnknownEntry {
                name: entry.to_owned(),
            });
        }
        let expression = self
            .rules
            .get(entry)
            .expect("validated public rules refer to an existing rule");
        let mut failure = FailureAccumulator::default();
        let initial = State {
            position: 0,
            captures: BTreeMap::new(),
        };
        let outcomes = self.evaluate(expression, initial, tokens, adapter, &mut failure);
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

    fn evaluate<T>(
        &self,
        expression: &Expr,
        state: State,
        tokens: &[T],
        adapter: &impl InputAdapter<T>,
        failure: &mut FailureAccumulator,
    ) -> Vec<Outcome> {
        match expression {
            Expr::Literal(literal) => {
                let position = state.position;
                match adapter.match_literal(tokens, position, literal) {
                    Some(value) if valid_terminal_match(&value, position, tokens.len()) => {
                        vec![Outcome::terminal(state.captures, value)]
                    }
                    _ => {
                        failure.record(position, format!("{literal:?}"));
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
                    failure.record(position, format!("@{name}"));
                }
                outcomes
            }
            Expr::RuleRef(name) => {
                let expression = self
                    .rules
                    .get(name)
                    .expect("validated rule references exist");
                self.evaluate(expression, state, tokens, adapter, failure)
            }
            Expr::Sequence(expressions) => {
                let mut outcomes = vec![Outcome::empty(state)];
                for expression in expressions {
                    let mut next = Vec::new();
                    for outcome in outcomes {
                        let children =
                            self.evaluate(expression, outcome.as_state(), tokens, adapter, failure);
                        for child in children {
                            next.push(outcome.clone().append(child));
                        }
                    }
                    if next.is_empty() {
                        return Vec::new();
                    }
                    outcomes = next;
                }
                outcomes
            }
            Expr::Choice(expressions) => expressions
                .iter()
                .flat_map(|expression| {
                    self.evaluate(expression, state.clone(), tokens, adapter, failure)
                })
                .collect(),
            Expr::Optional(expression) => {
                let mut outcomes = vec![Outcome::empty(state.clone())];
                outcomes.extend(self.evaluate(expression, state, tokens, adapter, failure));
                outcomes
            }
            Expr::Repeat {
                expression,
                minimum,
            } => self.evaluate_repeat(expression, *minimum, state, tokens, adapter, failure),
            Expr::Capture { name, expression } => self
                .evaluate(expression, state, tokens, adapter, failure)
                .into_iter()
                .filter_map(|mut outcome| {
                    let captured = combine_values(&outcome.values)?;
                    outcome
                        .captures
                        .entry(name.clone())
                        .or_default()
                        .push(captured.clone());
                    outcome.values = vec![captured];
                    Some(outcome)
                })
                .collect(),
        }
    }

    fn evaluate_repeat<T>(
        &self,
        expression: &Expr,
        minimum: usize,
        state: State,
        tokens: &[T],
        adapter: &impl InputAdapter<T>,
        failure: &mut FailureAccumulator,
    ) -> Vec<Outcome> {
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
                for child in self.evaluate(expression, outcome.as_state(), tokens, adapter, failure)
                {
                    if child.position <= outcome.position {
                        continue;
                    }
                    let combined = outcome.clone().append(child);
                    let next_count = count + 1;
                    if next_count >= minimum {
                        accepted.push(combined.clone());
                    }
                    next.push((next_count, combined));
                }
            }
            frontier = next;
        }
        accepted
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

#[derive(Default)]
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
        );
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
        );
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
        );
        assert_eq!(
            grammar.parse("entry", &["same"], &Words),
            Err(ParseError::Ambiguous { alternatives: 2 })
        );
    }
}
