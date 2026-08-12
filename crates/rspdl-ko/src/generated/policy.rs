use rspdl_grammar_compiler::{Capture, Grammar, ParseError};

use crate::scanner::Token;

use super::adapter::KoreanTokenAdapter;
use super::required_capture;

include!(concat!(env!("OUT_DIR"), "/policy_grammar.rs"));

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GeneratedPolicy {
    pub role: Capture,
    pub model: Capture,
    pub field: Capture,
    pub action: Capture,
    pub effect: Capture,
}

pub(crate) fn parse_policy(tokens: &[Token]) -> Result<GeneratedPolicy, ParseError> {
    let grammar: Grammar = generated_policy_grammar();
    let parsed = grammar.parse("policy_statement", tokens, &KoreanTokenAdapter)?;
    Ok(GeneratedPolicy {
        role: required_capture(&parsed, "role"),
        model: required_capture(&parsed, "model"),
        field: required_capture(&parsed, "field"),
        action: required_capture(&parsed, "action"),
        effect: required_capture(&parsed, "effect"),
    })
}

#[cfg(test)]
mod tests {
    use crate::ast::{DeclarationAst, PolicyEffectAst};
    use crate::scanner::TokenKind;
    use crate::{Diagnostic, parse, scan};

    use super::*;

    fn policy_tokens(sentence: &str) -> Vec<Token> {
        let scanned = scan(sentence);
        assert!(scanned.diagnostics.is_empty(), "{:?}", scanned.diagnostics);
        scanned
            .tokens
            .into_iter()
            .filter(|token| {
                !matches!(
                    token.kind,
                    TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent
                )
            })
            .collect()
    }

    fn handwritten_policy(sentence: &str) -> Result<crate::PolicyAst, Vec<Diagnostic>> {
        let source = format!(
            "@모듈 승인(expense)\n신청(request)은 다음 필드들로 구성되어 있다.\n    상태(status): 필수 문자열\n회계 관리자(accounting_manager)는 역할이다.\n변경(change)은 행동이다.\n{sentence}\n"
        );
        let parsed = parse(&source);
        if parsed.diagnostics.iter().any(Diagnostic::is_error) {
            return Err(parsed.diagnostics);
        }
        parsed
            .document
            .expect("valid source has a document")
            .declarations
            .into_iter()
            .find_map(|declaration| match declaration {
                DeclarationAst::Policy(policy) => Some(policy),
                _ => None,
            })
            .ok_or(parsed.diagnostics)
    }

    #[test]
    fn generated_policy_matches_handwritten_ast_captures() {
        let cases = [
            "회계 관리자는 신청의 상태를 변경할 수 있다.",
            "사용자은 신청의 상태를 변경할 수 없다.",
            "`회계 관리자` 는 `비용 신청` 의 `승인 상태` 를 `상태 변경` 할 수 있다.",
            "아이는 신청의 분류를 조회할 수 있다.",
        ];
        for sentence in cases {
            let handwritten = handwritten_policy(sentence)
                .unwrap_or_else(|diagnostics| panic!("{sentence}: {diagnostics:?}"));
            let generated = parse_policy(&policy_tokens(sentence))
                .unwrap_or_else(|error| panic!("{sentence}: {error:?}"));
            assert_eq!(generated.role.value, handwritten.role, "{sentence}");
            assert_eq!(generated.model.value, handwritten.model, "{sentence}");
            assert_eq!(generated.field.value, handwritten.field, "{sentence}");
            assert_eq!(generated.action.value, handwritten.action, "{sentence}");
            assert_eq!(
                generated.effect.value,
                match handwritten.effect {
                    PolicyEffectAst::Allow => "있다",
                    PolicyEffectAst::Deny => "없다",
                },
                "{sentence}"
            );
        }
    }

    #[test]
    fn generated_policy_rejects_every_shape_rejected_by_the_oracle() {
        let cases = [
            "회계 관리자가 신청의 상태를 변경할 수 있다.",
            "회계 관리자는 신청의 상태를 변경할 있다.",
            "회계 관리자는 신청의 상태를 변경할 수 모른다.",
            "회계 관리자는 신청의 상태를 변경할 수 있다 뒤에.",
            "회계 관리자는 신청의 상태를 변경할 수 있다",
        ];
        for sentence in cases {
            assert!(handwritten_policy(sentence).is_err(), "{sentence}");
            assert!(
                parse_policy(&policy_tokens(sentence)).is_err(),
                "{sentence}"
            );
        }
    }

    #[test]
    fn generated_capture_spans_exclude_attached_markers() {
        let sentence = "회계 관리자는 신청의 상태를 변경할 수 있다.";
        let generated = parse_policy(&policy_tokens(sentence)).unwrap();
        assert_eq!(
            &sentence[generated.role.start..generated.role.end],
            "회계 관리자"
        );
        assert_eq!(
            &sentence[generated.action.start..generated.action.end],
            "변경"
        );
    }
}
