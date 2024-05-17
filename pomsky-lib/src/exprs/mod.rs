use crate::{
    capturing_groups::CapturingGroupsCollector,
    compile::{CompileResult, CompileState},
    diagnose::Diagnostic,
    options::CompileOptions,
    regex::Count,
    validation::Validator,
    visitor::RuleVisitor,
};

pub(crate) mod alternation;
pub(crate) mod boundary;
pub(crate) mod char_class;
pub(crate) mod codepoint;
pub(crate) mod dot;
pub(crate) mod grapheme;
pub(crate) mod group;
pub(crate) mod literal;
pub(crate) mod lookaround;
pub(crate) mod range;
pub(crate) mod recursion;
pub(crate) mod reference;
pub(crate) mod regex;
pub(crate) mod repetition;
pub(crate) mod rule;
pub(crate) mod stmt;
pub(crate) mod var;

use pomsky_syntax::exprs::{test::Test, *};
use pomsky_syntax::Span;

pub(crate) trait RuleExt<'i> {
    fn compile<'c>(
        &'c self,
        options: CompileOptions,
        state: &mut CompileState<'c, 'i>,
    ) -> CompileResult<'i>;
}

/// A parsed pomsky expression, which might contain more sub-expressions.
#[derive(Clone)]
#[cfg_attr(not(feature = "dbg"), derive(Debug))]
pub struct Expr<'i>(Rule<'i>);

impl<'i> Expr<'i> {
    /// Parse a `Expr` without generating code.
    ///
    /// The parsed `Expr` can be displayed with `Debug` if the `dbg` feature is
    /// enabled.
    pub fn parse(input: &'i str) -> (Option<Self>, impl Iterator<Item = Diagnostic> + '_) {
        let (rule, diagnostics) = pomsky_syntax::parse(input, 256);
        (rule.map(Expr), diagnostics.into_iter().map(|d| Diagnostic::from_parser(&d, input)))
    }

    /// Compile a `Expr` that has been parsed, to a regex
    pub fn compile(
        &self,
        input: &'i str,
        options: CompileOptions,
    ) -> (Option<String>, Vec<Diagnostic>) {
        if let Err(e) = Validator::new(options).visit_rule(&self.0) {
            return (None, vec![e.diagnostic(input)]);
        }

        let mut capt_groups = CapturingGroupsCollector::new();
        if let Err(e) = capt_groups.visit_rule(&self.0) {
            return (None, vec![e.diagnostic(input)]);
        }

        let no_span = Span::empty();

        let start = Rule::Boundary(Boundary::new(BoundaryKind::Start, true, no_span));
        let end = Rule::Boundary(Boundary::new(BoundaryKind::End, true, no_span));
        let grapheme = Rule::Grapheme;
        let codepoint = Rule::Codepoint;

        let builtins = vec![
            ("Start", &start),
            ("End", &end),
            ("Grapheme", &grapheme),
            ("G", &grapheme),
            ("Codepoint", &codepoint),
            ("C", &codepoint),
        ];

        let mut state = CompileState::new(capt_groups, builtins);
        let mut compiled = match self.0.compile(options, &mut state) {
            Ok(compiled) => compiled,
            Err(e) => return (None, vec![e.diagnostic(input)]),
        };
        let count = compiled.optimize();

        let mut buf = String::new();
        if count != Count::Zero {
            compiled.codegen(&mut buf, options.flavor);
        }
        (Some(buf), state.diagnostics)
    }

    /// Extracts top-level all unit tests from the Pomsky expression
    pub fn extract_tests(self) -> Vec<Test<'i>> {
        let mut rule = self.0;
        let mut tests = Vec::new();
        while let Rule::StmtExpr(expr) = rule {
            if let Stmt::Test(test) = expr.stmt {
                tests.push(test);
            }
            rule = expr.rule;
        }
        tests
    }

    /// Parse a string to a `Expr` and compile it to a regex.
    pub fn parse_and_compile(
        input: &'i str,
        options: CompileOptions,
    ) -> (Option<String>, Vec<Diagnostic>, Vec<Test<'i>>) {
        match Self::parse(input) {
            (Some(parsed), warnings1) => match parsed.compile(input, options) {
                (Some(compiled), warnings2) => {
                    let mut diagnostics =
                        Vec::with_capacity(warnings1.size_hint().0 + warnings2.len());
                    diagnostics.extend(warnings1);
                    diagnostics.extend(warnings2);
                    (Some(compiled), diagnostics, parsed.extract_tests())
                }
                (None, errors) => {
                    let mut diagnostics =
                        Vec::with_capacity(warnings1.size_hint().0 + errors.len());
                    diagnostics.extend(errors);
                    diagnostics.extend(warnings1);
                    (None, diagnostics, parsed.extract_tests())
                }
            },
            (None, diagnostics) => (None, diagnostics.collect(), vec![]),
        }
    }
}

#[cfg(feature = "dbg")]
impl core::fmt::Debug for Expr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(&self.0, f)
    }
}
