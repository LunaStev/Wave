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

//! Read-only expression traversal shared by semantic analysis and HIR.
use super::{ASTNode, Expression, StatementNode};

pub fn walk_nodes(nodes: &[ASTNode], visit: &mut impl FnMut(&Expression)) {
    for node in nodes {
        walk_node(node, visit);
    }
}

pub fn walk_node(node: &ASTNode, visit: &mut impl FnMut(&Expression)) {
    match node {
        ASTNode::Located { value, .. } => walk_node(value, visit),
        ASTNode::Function(function) => {
            for parameter in &function.parameters {
                if let Some(default) = &parameter.initial_value {
                    walk_expression(default, visit);
                }
            }
            walk_nodes(&function.body, visit);
        }
        ASTNode::Struct(structure) => {
            for method in &structure.methods {
                for parameter in &method.parameters {
                    if let Some(default) = &parameter.initial_value {
                        walk_expression(default, visit);
                    }
                }
                walk_nodes(&method.body, visit);
            }
        }
        ASTNode::ProtoImpl(implementation) => {
            for method in &implementation.methods {
                for parameter in &method.parameters {
                    if let Some(default) = &parameter.initial_value {
                        walk_expression(default, visit);
                    }
                }
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

pub fn walk_expression(expression: &Expression, visit: &mut impl FnMut(&Expression)) {
    if let Expression::Located { value, .. } = expression {
        walk_expression(value, visit);
        return;
    }
    visit(expression);
    match expression {
        Expression::Located { value, .. } => walk_expression(value, visit),
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
        | Expression::Await(inner)
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
