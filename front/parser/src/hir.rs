// This file is part of the Wave language project.
// Copyright (c) 2024–2026 Wave Foundation
// Copyright (c) 2024–2026 LunaStev and contributors
//
// This Source Code Form is subject to the terms of the
// Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file,
// You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0
// AI TRAINING NOTICE: Prohibited without prior written permission. No use for machine learning or generative AI training, fine-tuning, distillation, embedding, or dataset creation.

//! Backend-neutral typed frontend program.
//!
//! [`TypedProgram`] is the boundary after import expansion, generic
//! monomorphization, and semantic validation. It owns the final source AST in
//! stable storage and assigns every expression a stable [`ExpressionId`]. This
//! lets future variant and async lowering attach semantic facts without using
//! backend-owned state or expression addresses as public identities.

use crate::ast::{ASTNode, Expression, MatchPattern, StatementNode, WaveType};
use crate::verification::{analyze_hir_expression_types, SemanticDiagnostic};
use std::collections::HashMap;
use std::fmt;

/// Stable identity of an expression within one [`TypedProgram`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ExpressionId(usize);

impl ExpressionId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// Stable identity of a match pattern within one [`TypedProgram`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PatternId(usize);

impl PatternId {
    pub fn index(self) -> usize {
        self.0
    }
}

/// The semantic type known before contextual lowering is performed.
///
/// Literal forms remain explicit because their final representation can depend
/// on an assignment, argument, return, or aggregate context. They are not
/// silently committed to a backend type at this boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HirExpressionType {
    Resolved(WaveType),
    IntegerLiteral,
    FloatLiteral,
    Null,
    ArrayLiteral,
    AddressedArrayLiteral,
    Unknown,
}

/// Fully resolved variant constructor selected by semantic analysis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirVariantConstruction {
    pub variant_type: WaveType,
    pub case_name: String,
    pub discriminant: u32,
    pub payload_types: Vec<WaveType>,
}

/// Concrete variant case selected by a semantically validated pattern.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HirVariantPattern {
    pub variant_type: WaveType,
    pub case_name: String,
    pub discriminant: u32,
    pub payload_types: Vec<WaveType>,
}

/// Semantically validated frontend program consumed by later lowering passes.
///
/// The syntax is boxed before analysis, so moving `TypedProgram` never changes
/// expression addresses. Addresses are only an internal lookup optimization;
/// consumers observe stable `ExpressionId` values.
#[derive(Debug)]
pub struct TypedProgram {
    syntax: Box<[ASTNode]>,
    expression_ids: HashMap<usize, ExpressionId>,
    expression_types: Vec<HirExpressionType>,
    variant_constructions: Vec<Option<HirVariantConstruction>>,
    pattern_ids: HashMap<usize, PatternId>,
    variant_patterns: Vec<Option<HirVariantPattern>>,
}

/// Semantic lowering failure that retains the syntax used for source mapping.
#[derive(Debug)]
pub struct HirLoweringError {
    syntax: Box<[ASTNode]>,
    diagnostic: SemanticDiagnostic,
}

impl HirLoweringError {
    pub fn syntax(&self) -> &[ASTNode] {
        &self.syntax
    }

    pub fn diagnostic(&self) -> &SemanticDiagnostic {
        &self.diagnostic
    }

    pub fn into_parts(self) -> (Box<[ASTNode]>, SemanticDiagnostic) {
        (self.syntax, self.diagnostic)
    }
}

impl fmt::Display for HirLoweringError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.diagnostic.fmt(formatter)
    }
}

impl std::error::Error for HirLoweringError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.diagnostic)
    }
}

impl TypedProgram {
    /// Validates a final AST and builds its stable typed frontend representation.
    pub fn lower(syntax: Vec<ASTNode>) -> Result<Self, HirLoweringError> {
        let syntax = syntax.into_boxed_slice();
        let (analyzed_types, analyzed_variants, analyzed_patterns) =
            match analyze_hir_expression_types(&syntax) {
                Ok(analysis) => analysis,
                Err(diagnostic) => return Err(HirLoweringError { syntax, diagnostic }),
            };
        let mut expression_ids = HashMap::with_capacity(analyzed_types.len());
        let mut expression_types = Vec::with_capacity(analyzed_types.len());
        let mut variant_constructions = Vec::with_capacity(analyzed_variants.len());

        walk_nodes(&syntax, &mut |expression| {
            let address = expression as *const Expression as usize;
            let id = ExpressionId(expression_types.len());
            expression_ids.insert(address, id);
            expression_types.push(
                analyzed_types
                    .get(&address)
                    .cloned()
                    .unwrap_or(HirExpressionType::Unknown),
            );
            variant_constructions.push(analyzed_variants.get(&address).cloned());
        });

        let mut pattern_ids = HashMap::with_capacity(analyzed_patterns.len());
        let mut variant_patterns = Vec::with_capacity(analyzed_patterns.len());
        walk_patterns_in_nodes(&syntax, &mut |pattern| {
            let address = pattern as *const MatchPattern as usize;
            let id = PatternId(variant_patterns.len());
            pattern_ids.insert(address, id);
            variant_patterns.push(analyzed_patterns.get(&address).cloned());
        });

        Ok(Self {
            syntax,
            expression_ids,
            expression_types,
            variant_constructions,
            pattern_ids,
            variant_patterns,
        })
    }

    pub fn syntax(&self) -> &[ASTNode] {
        &self.syntax
    }

    pub fn expression_count(&self) -> usize {
        self.expression_types.len()
    }

    pub fn expression_id(&self, expression: &Expression) -> Option<ExpressionId> {
        self.expression_ids
            .get(&(expression as *const Expression as usize))
            .copied()
    }

    pub fn expression_type(&self, id: ExpressionId) -> Option<&HirExpressionType> {
        self.expression_types.get(id.index())
    }

    pub fn type_of(&self, expression: &Expression) -> Option<&HirExpressionType> {
        self.expression_id(expression)
            .and_then(|id| self.expression_type(id))
    }

    pub fn variant_construction(&self, id: ExpressionId) -> Option<&HirVariantConstruction> {
        self.variant_constructions.get(id.index())?.as_ref()
    }

    pub fn pattern_id(&self, pattern: &MatchPattern) -> Option<PatternId> {
        self.pattern_ids
            .get(&(pattern as *const MatchPattern as usize))
            .copied()
    }

    pub fn variant_pattern(&self, id: PatternId) -> Option<&HirVariantPattern> {
        self.variant_patterns.get(id.index())?.as_ref()
    }
}

fn walk_nodes(nodes: &[ASTNode], visit: &mut impl FnMut(&Expression)) {
    for node in nodes {
        walk_node(node, visit);
    }
}

fn walk_node(node: &ASTNode, visit: &mut impl FnMut(&Expression)) {
    match node {
        ASTNode::Function(function) => walk_nodes(&function.body, visit),
        ASTNode::Struct(structure) => {
            for method in &structure.methods {
                walk_nodes(&method.body, visit);
            }
        }
        ASTNode::ProtoImpl(implementation) => {
            for method in &implementation.methods {
                walk_nodes(&method.body, visit);
            }
        }
        ASTNode::Statement(statement) => walk_statement(statement, visit),
        ASTNode::Variable(variable) => {
            if let Some(initializer) = &variable.initial_value {
                walk_expression(initializer, visit);
            }
        }
        ASTNode::Expression(expression) => walk_expression(expression, visit),
        ASTNode::ExternFunction(_)
        | ASTNode::Program(_)
        | ASTNode::TypeAlias(_)
        | ASTNode::Enum(_)
        | ASTNode::Variant(_) => {}
    }
}

fn walk_statement(statement: &StatementNode, visit: &mut impl FnMut(&Expression)) {
    match statement {
        StatementNode::PrintFormat { args, .. }
        | StatementNode::PrintlnFormat { args, .. }
        | StatementNode::Input { args, .. } => {
            for argument in args {
                walk_expression(argument, visit);
            }
        }
        StatementNode::If {
            condition,
            body,
            else_if_blocks,
            else_block,
        } => {
            walk_expression(condition, visit);
            walk_nodes(body, visit);
            if let Some(blocks) = else_if_blocks {
                for (condition, body) in blocks.iter() {
                    walk_expression(condition, visit);
                    walk_nodes(body, visit);
                }
            }
            if let Some(body) = else_block {
                walk_nodes(body, visit);
            }
        }
        StatementNode::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            walk_node(initialization, visit);
            walk_expression(condition, visit);
            walk_expression(increment, visit);
            walk_nodes(body, visit);
        }
        StatementNode::While { condition, body } => {
            walk_expression(condition, visit);
            walk_nodes(body, visit);
        }
        StatementNode::Match { value, arms } => {
            walk_expression(value, visit);
            for arm in arms {
                walk_nodes(&arm.body, visit);
            }
        }
        StatementNode::Assign { value, .. } => walk_expression(value, visit),
        StatementNode::AsmBlock {
            inputs, outputs, ..
        } => {
            for (_, expression) in inputs.iter().chain(outputs.iter()) {
                walk_expression(expression, visit);
            }
        }
        StatementNode::Return(Some(expression)) | StatementNode::Expression(expression) => {
            walk_expression(expression, visit)
        }
        StatementNode::Print(_)
        | StatementNode::Println(_)
        | StatementNode::Variable(_)
        | StatementNode::Import(_)
        | StatementNode::Break
        | StatementNode::Continue
        | StatementNode::Return(None) => {}
    }
}

fn walk_expression(expression: &Expression, visit: &mut impl FnMut(&Expression)) {
    visit(expression);
    match expression {
        Expression::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                walk_expression(value, visit);
            }
        }
        Expression::FunctionCall { args, .. } => {
            for argument in args {
                walk_expression(argument, visit);
            }
        }
        Expression::MethodCall { object, args, .. } => {
            walk_expression(object, visit);
            for argument in args {
                walk_expression(argument, visit);
            }
        }
        Expression::Deref(inner)
        | Expression::AddressOf(inner)
        | Expression::Grouped(inner)
        | Expression::Unary { expr: inner, .. }
        | Expression::Cast { expr: inner, .. }
        | Expression::FieldAccess { object: inner, .. }
        | Expression::IncDec { target: inner, .. } => walk_expression(inner, visit),
        Expression::BinaryExpression { left, right, .. }
        | Expression::IndexAccess {
            target: left,
            index: right,
        }
        | Expression::AssignOperation {
            target: left,
            value: right,
            ..
        }
        | Expression::Assignment {
            target: left,
            value: right,
        } => {
            walk_expression(left, visit);
            walk_expression(right, visit);
        }
        Expression::ArrayLiteral(values) => {
            for value in values {
                walk_expression(value, visit);
            }
        }
        Expression::AsmBlock {
            inputs, outputs, ..
        } => {
            for (_, expression) in inputs.iter().chain(outputs.iter()) {
                walk_expression(expression, visit);
            }
        }
        Expression::Null | Expression::Literal(_) | Expression::Variable(_) => {}
    }
}

fn walk_patterns_in_nodes(nodes: &[ASTNode], visit: &mut impl FnMut(&MatchPattern)) {
    for node in nodes {
        match node {
            ASTNode::Function(function) => walk_patterns_in_nodes(&function.body, visit),
            ASTNode::Struct(structure) => {
                for method in &structure.methods {
                    walk_patterns_in_nodes(&method.body, visit);
                }
            }
            ASTNode::ProtoImpl(implementation) => {
                for method in &implementation.methods {
                    walk_patterns_in_nodes(&method.body, visit);
                }
            }
            ASTNode::Statement(statement) => walk_patterns_in_statement(statement, visit),
            ASTNode::ExternFunction(_)
            | ASTNode::Program(_)
            | ASTNode::Variable(_)
            | ASTNode::Expression(_)
            | ASTNode::TypeAlias(_)
            | ASTNode::Enum(_)
            | ASTNode::Variant(_) => {}
        }
    }
}

fn walk_patterns_in_statement(statement: &StatementNode, visit: &mut impl FnMut(&MatchPattern)) {
    match statement {
        StatementNode::If {
            body,
            else_if_blocks,
            else_block,
            ..
        } => {
            walk_patterns_in_nodes(body, visit);
            if let Some(blocks) = else_if_blocks {
                for (_, body) in blocks.iter() {
                    walk_patterns_in_nodes(body, visit);
                }
            }
            if let Some(body) = else_block {
                walk_patterns_in_nodes(body, visit);
            }
        }
        StatementNode::For {
            initialization,
            body,
            ..
        } => {
            walk_patterns_in_nodes(std::slice::from_ref(initialization.as_ref()), visit);
            walk_patterns_in_nodes(body, visit);
        }
        StatementNode::While { body, .. } => walk_patterns_in_nodes(body, visit),
        StatementNode::Match { arms, .. } => {
            for arm in arms {
                walk_pattern(&arm.pattern, visit);
                walk_patterns_in_nodes(&arm.body, visit);
            }
        }
        StatementNode::Print(_)
        | StatementNode::PrintFormat { .. }
        | StatementNode::Println(_)
        | StatementNode::PrintlnFormat { .. }
        | StatementNode::Input { .. }
        | StatementNode::Variable(_)
        | StatementNode::Import(_)
        | StatementNode::Assign { .. }
        | StatementNode::AsmBlock { .. }
        | StatementNode::Break
        | StatementNode::Continue
        | StatementNode::Return(_)
        | StatementNode::Expression(_) => {}
    }
}

fn walk_pattern(pattern: &MatchPattern, visit: &mut impl FnMut(&MatchPattern)) {
    visit(pattern);
    if let MatchPattern::Variant { payloads, .. } = pattern {
        for payload in payloads {
            walk_pattern(payload, visit);
        }
    }
}
