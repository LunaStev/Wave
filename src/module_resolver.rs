//! Import graph construction, module visibility, and namespace lowering.
//!
//! Wave's backend still consumes one concrete AST. This pass preserves module
//! boundaries while resolving imports, then gives every imported declaration a
//! collision-free internal name before the existing semantic and LLVM phases.

use ::error::{WaveError, WaveErrorKind};
use ::parser::ast::*;
use ::parser::import::{local_import_unit_with_config, ImportConfig};
use ::parser::types::{parse_type, split_top_level_generic_args, token_type_to_wave_type};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ModuleSource {
    pub path: PathBuf,
    pub source: String,
}

#[derive(Debug)]
pub struct ResolvedModuleGraph {
    pub ast: Vec<ASTNode>,
    pub origins: Vec<usize>,
    pub sources: Vec<ModuleSource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolKind {
    Function,
    Struct,
    VariantConstructor,
    Type,
    Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModuleSymbol {
    lowered: String,
    visibility: Visibility,
    kind: SymbolKind,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ModuleInterface {
    symbols: HashMap<String, ModuleSymbol>,
}

#[derive(Default)]
struct NameContext {
    own: HashMap<String, ModuleSymbol>,
    selected: HashMap<String, ModuleSymbol>,
    namespaces: HashMap<String, ModuleInterface>,
}

struct Resolver<'a> {
    config: &'a ImportConfig,
    interfaces: HashMap<PathBuf, ModuleInterface>,
    visiting: Vec<PathBuf>,
    source_indices: HashMap<PathBuf, usize>,
    ast: Vec<ASTNode>,
    origins: Vec<usize>,
    sources: Vec<ModuleSource>,
}

pub fn resolve_import_graph(
    entry_path: &Path,
    entry_source: &str,
    ast: Vec<ASTNode>,
    config: &ImportConfig,
) -> Result<ResolvedModuleGraph, WaveError> {
    let entry_key = canonical_key(entry_path);
    let mut resolver = Resolver {
        config,
        interfaces: HashMap::new(),
        visiting: Vec::new(),
        source_indices: HashMap::from([(entry_key.clone(), 0)]),
        ast: Vec::new(),
        origins: Vec::new(),
        sources: vec![ModuleSource {
            path: entry_path.to_path_buf(),
            source: entry_source.to_string(),
        }],
    };

    resolver.resolve_module(entry_key.clone(), ast, 0, true)?;
    infer_variable_types(&mut resolver.ast).map_err(|message| {
        module_error(
            &entry_key,
            "Type inference failed",
            message,
            "add an explicit `: type` annotation to the variable",
        )
    })?;
    Ok(ResolvedModuleGraph {
        ast: resolver.ast,
        origins: resolver.origins,
        sources: resolver.sources,
    })
}

#[derive(Clone)]
struct CallableType {
    generic_params: Vec<String>,
    return_type: WaveType,
}

#[derive(Default)]
struct InferenceTypes {
    functions: HashMap<String, CallableType>,
    methods: HashMap<(String, String), CallableType>,
    fields: HashMap<String, HashMap<String, WaveType>>,
    globals: HashMap<String, WaveType>,
    variants: HashMap<String, InferenceVariantConstructor>,
}

#[derive(Clone)]
struct InferenceVariantConstructor {
    owner: String,
    generic_params: Vec<String>,
    payload_types: Vec<WaveType>,
}

fn infer_variable_types(ast: &mut [ASTNode]) -> Result<(), String> {
    let mut types = InferenceTypes::default();
    for node in ast.iter() {
        match node {
            ASTNode::Function(function) => {
                types.functions.insert(
                    function.name.clone(),
                    CallableType {
                        generic_params: function.generic_params.clone(),
                        return_type: function.return_type.clone().unwrap_or(WaveType::Void),
                    },
                );
            }
            ASTNode::ExternFunction(function) => {
                types.functions.insert(
                    function.name.clone(),
                    CallableType {
                        generic_params: Vec::new(),
                        return_type: function.return_type.clone(),
                    },
                );
            }
            ASTNode::Struct(structure) => {
                types.fields.insert(
                    structure.name.clone(),
                    structure.fields.iter().cloned().collect(),
                );
                for method in &structure.methods {
                    types.methods.insert(
                        (structure.name.clone(), method.name.clone()),
                        CallableType {
                            generic_params: method.generic_params.clone(),
                            return_type: method.return_type.clone().unwrap_or(WaveType::Void),
                        },
                    );
                }
            }
            ASTNode::Variant(variant) => {
                for case in &variant.cases {
                    types.variants.insert(
                        format!("{}::{}", variant.name, case.name),
                        InferenceVariantConstructor {
                            owner: variant.name.clone(),
                            generic_params: variant.generic_params.clone(),
                            payload_types: case.payload_types.clone(),
                        },
                    );
                }
            }
            ASTNode::Variable(variable) => {
                types
                    .globals
                    .insert(variable.name.clone(), variable.type_name.clone());
            }
            _ => {}
        }
    }

    for node in ast.iter_mut() {
        if let ASTNode::Variable(variable) = node {
            if variable.type_name == WaveType::Infer {
                let initializer = variable.initial_value.as_ref().ok_or_else(|| {
                    format!(
                        "cannot infer type of global variable '{}' without an initializer",
                        variable.name
                    )
                })?;
                variable.type_name = infer_expression_type(initializer, &types, &HashMap::new())?;
                types
                    .globals
                    .insert(variable.name.clone(), variable.type_name.clone());
            }
        }
    }

    for node in ast.iter_mut() {
        match node {
            ASTNode::Function(function) => infer_function_body(function, &types)?,
            ASTNode::Struct(structure) => {
                for method in &mut structure.methods {
                    infer_function_body(method, &types)?;
                }
            }
            ASTNode::ProtoImpl(implementation) => {
                for method in &mut implementation.methods {
                    infer_function_body(method, &types)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn infer_function_body(function: &mut FunctionNode, types: &InferenceTypes) -> Result<(), String> {
    let mut locals = function
        .parameters
        .iter()
        .map(|parameter| (parameter.name.clone(), parameter.param_type.clone()))
        .collect::<HashMap<_, _>>();
    infer_block(&mut function.body, types, &mut locals)
}

fn infer_block(
    nodes: &mut [ASTNode],
    types: &InferenceTypes,
    locals: &mut HashMap<String, WaveType>,
) -> Result<(), String> {
    for node in nodes {
        match node {
            ASTNode::Variable(variable) => {
                if variable.type_name == WaveType::Infer {
                    let initializer = variable.initial_value.as_ref().ok_or_else(|| {
                        format!(
                            "cannot infer type of variable '{}' without an initializer",
                            variable.name
                        )
                    })?;
                    variable.type_name = infer_expression_type(initializer, types, locals)
                        .map_err(|reason| {
                            format!(
                                "cannot infer type of variable '{}': {}",
                                variable.name, reason
                            )
                        })?;
                }
                locals.insert(variable.name.clone(), variable.type_name.clone());
            }
            ASTNode::Statement(statement) => infer_statement(statement, types, locals)?,
            _ => {}
        }
    }
    Ok(())
}

fn infer_statement(
    statement: &mut StatementNode,
    types: &InferenceTypes,
    locals: &mut HashMap<String, WaveType>,
) -> Result<(), String> {
    match statement {
        StatementNode::If {
            body,
            else_if_blocks,
            else_block,
            ..
        } => {
            let mut scope = locals.clone();
            infer_block(body, types, &mut scope)?;
            if let Some(blocks) = else_if_blocks {
                for (_, body) in blocks.iter_mut() {
                    let mut scope = locals.clone();
                    infer_block(body, types, &mut scope)?;
                }
            }
            if let Some(body) = else_block {
                let mut scope = locals.clone();
                infer_block(body, types, &mut scope)?;
            }
        }
        StatementNode::For {
            initialization,
            body,
            ..
        } => {
            let mut scope = locals.clone();
            infer_block(
                std::slice::from_mut(initialization.as_mut()),
                types,
                &mut scope,
            )?;
            infer_block(body, types, &mut scope)?;
        }
        StatementNode::While { body, .. } => {
            let mut scope = locals.clone();
            infer_block(body, types, &mut scope)?;
        }
        StatementNode::Match { arms, .. } => {
            for arm in arms {
                let mut scope = locals.clone();
                infer_block(&mut arm.body, types, &mut scope)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn infer_expression_type(
    expression: &Expression,
    types: &InferenceTypes,
    locals: &HashMap<String, WaveType>,
) -> Result<WaveType, String> {
    match expression {
        Expression::Literal(Literal::Int(_)) => Ok(WaveType::Int(32)),
        Expression::Literal(Literal::Float(_)) => Ok(WaveType::Float(64)),
        Expression::Literal(Literal::String(_)) => Ok(WaveType::String),
        Expression::Literal(Literal::Bool(_)) => Ok(WaveType::Bool),
        Expression::Literal(Literal::Char(_)) => Ok(WaveType::Char),
        Expression::Literal(Literal::Byte(_)) => Ok(WaveType::Byte),
        Expression::Null => Err("`null` needs an explicit pointer type".to_string()),
        Expression::Variable(name) => {
            if let Some(ty) = locals.get(name).or_else(|| types.globals.get(name)) {
                Ok(ty.clone())
            } else if types.variants.contains_key(name) {
                infer_variant_constructor_type(name, &[], types, locals)
            } else {
                Err(format!("unknown value '{}'", name))
            }
        }
        Expression::StructLiteral { name, .. } => Ok(WaveType::Struct(name.clone())),
        Expression::FunctionCall {
            name, type_args, ..
        } => {
            if let Some(constructor) = types.variants.get(name) {
                let Expression::FunctionCall { args, .. } = expression else {
                    unreachable!()
                };
                return infer_variant_constructor_type_from(name, constructor, args, types, locals);
            }
            let callable = types
                .functions
                .get(name)
                .ok_or_else(|| format!("unknown function '{}'", name))?;
            Ok(substitute_callable_return(callable, type_args))
        }
        Expression::MethodCall { object, name, .. } => {
            let object_type = infer_expression_type(object, types, locals)?;
            let WaveType::Struct(struct_name) = object_type else {
                return Err(format!("method '{}' receiver type is not a struct", name));
            };
            let callable = types
                .methods
                .get(&(struct_name.clone(), name.clone()))
                .ok_or_else(|| format!("unknown method '{}.{}'", struct_name, name))?;
            Ok(callable.return_type.clone())
        }
        Expression::Deref(inner) => match infer_expression_type(inner, types, locals)? {
            WaveType::Pointer(inner) => Ok(*inner),
            other => Err(format!("cannot dereference {:?}", other)),
        },
        Expression::AddressOf(inner) => Ok(WaveType::Pointer(Box::new(infer_expression_type(
            inner, types, locals,
        )?))),
        Expression::BinaryExpression {
            left,
            operator,
            right,
        } => {
            if matches!(
                operator,
                Operator::GreaterEqual
                    | Operator::LessEqual
                    | Operator::Greater
                    | Operator::Less
                    | Operator::Equal
                    | Operator::NotEqual
                    | Operator::LogicalAnd
                    | Operator::LogicalOr
                    | Operator::LogicalNot
                    | Operator::Not
            ) {
                return Ok(WaveType::Bool);
            }
            let left_literal = expression_literal_kind(left);
            let right_literal = expression_literal_kind(right);
            let left = infer_expression_type(left, types, locals)?;
            let right = infer_expression_type(right, types, locals)?;
            if left == right {
                Ok(left)
            } else if matches!(left_literal, Some(false)) && is_integer_type(&right) {
                Ok(right)
            } else if matches!(right_literal, Some(false)) && is_integer_type(&left) {
                Ok(left)
            } else if matches!(left_literal, Some(true)) && matches!(&right, WaveType::Float(_)) {
                Ok(right)
            } else if matches!(right_literal, Some(true)) && matches!(&left, WaveType::Float(_)) {
                Ok(left)
            } else {
                Err(format!(
                    "binary operands have different types {:?} and {:?}",
                    left, right
                ))
            }
        }
        Expression::IndexAccess { target, .. } => {
            match infer_expression_type(target, types, locals)? {
                WaveType::Array(inner, _) | WaveType::Pointer(inner) => Ok(*inner),
                WaveType::String => Ok(WaveType::Byte),
                other => Err(format!("cannot index {:?}", other)),
            }
        }
        Expression::ArrayLiteral(values) => {
            let first = values
                .first()
                .ok_or_else(|| "empty array literal needs an explicit type".to_string())?;
            let element = infer_expression_type(first, types, locals)?;
            Ok(WaveType::Array(Box::new(element), values.len() as u32))
        }
        Expression::Grouped(inner) => infer_expression_type(inner, types, locals),
        Expression::AssignOperation { target, .. }
        | Expression::Assignment { target, .. }
        | Expression::IncDec { target, .. } => infer_expression_type(target, types, locals),
        Expression::FieldAccess { object, field } => {
            let WaveType::Struct(struct_name) = infer_expression_type(object, types, locals)?
            else {
                return Err(format!("field '{}' receiver is not a struct", field));
            };
            types
                .fields
                .get(&struct_name)
                .and_then(|fields| fields.get(field))
                .cloned()
                .ok_or_else(|| format!("unknown field '{}.{}'", struct_name, field))
        }
        Expression::Unary { operator, expr } => {
            if matches!(operator, Operator::Not | Operator::LogicalNot) {
                Ok(WaveType::Bool)
            } else {
                infer_expression_type(expr, types, locals)
            }
        }
        Expression::Cast { target_type, .. } => Ok(target_type.clone()),
        Expression::AsmBlock { .. } => {
            Err("inline assembly result needs an explicit type".to_string())
        }
    }
}

fn infer_variant_constructor_type(
    name: &str,
    args: &[Expression],
    types: &InferenceTypes,
    locals: &HashMap<String, WaveType>,
) -> Result<WaveType, String> {
    let constructor = types
        .variants
        .get(name)
        .ok_or_else(|| format!("unknown variant constructor '{}'", name))?;
    infer_variant_constructor_type_from(name, constructor, args, types, locals)
}

fn infer_variant_constructor_type_from(
    name: &str,
    constructor: &InferenceVariantConstructor,
    args: &[Expression],
    types: &InferenceTypes,
    locals: &HashMap<String, WaveType>,
) -> Result<WaveType, String> {
    if args.len() != constructor.payload_types.len() {
        return Err(format!(
            "variant constructor '{}' expects {} payload value(s), found {}",
            name,
            constructor.payload_types.len(),
            args.len()
        ));
    }
    if constructor.generic_params.is_empty() {
        return Ok(WaveType::Struct(constructor.owner.clone()));
    }

    let mut substitutions = HashMap::new();
    for (argument, template) in args.iter().zip(&constructor.payload_types) {
        let actual = infer_expression_type(argument, types, locals)?;
        infer_module_variant_substitution(
            template,
            &actual,
            &constructor.generic_params,
            &mut substitutions,
        )?;
    }
    let missing = constructor
        .generic_params
        .iter()
        .filter(|parameter| !substitutions.contains_key(*parameter))
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "cannot infer generic argument(s) {} for variant constructor '{}'",
            missing.join(", "),
            name
        ));
    }
    let arguments = constructor
        .generic_params
        .iter()
        .map(|parameter| module_type_name(&substitutions[parameter]))
        .collect::<Vec<_>>()
        .join(",");
    Ok(WaveType::Struct(format!(
        "{}<{}>",
        constructor.owner, arguments
    )))
}

fn infer_module_variant_substitution(
    template: &WaveType,
    actual: &WaveType,
    generic_params: &[String],
    substitutions: &mut HashMap<String, WaveType>,
) -> Result<(), String> {
    if let WaveType::Struct(name) = template {
        if generic_params.contains(name) {
            if let Some(previous) = substitutions.get(name) {
                if previous != actual {
                    return Err(format!(
                        "conflicting inferred types {:?} and {:?} for variant generic '{}'",
                        previous, actual, name
                    ));
                }
            } else {
                substitutions.insert(name.clone(), actual.clone());
            }
            return Ok(());
        }
    }
    match (template, actual) {
        (WaveType::Pointer(template), WaveType::Pointer(actual))
        | (WaveType::Array(template, _), WaveType::Array(actual, _)) => {
            infer_module_variant_substitution(template, actual, generic_params, substitutions)
        }
        (WaveType::Struct(template_name), WaveType::Struct(actual_name))
        | (WaveType::Struct(template_name), WaveType::Variant(actual_name))
        | (WaveType::Variant(template_name), WaveType::Struct(actual_name))
        | (WaveType::Variant(template_name), WaveType::Variant(actual_name)) => {
            let Some((template_base, template_args)) =
                parse_module_named_type_application(template_name)
            else {
                return Ok(());
            };
            let Some((actual_base, actual_args)) = parse_module_named_type_application(actual_name)
            else {
                return Ok(());
            };
            if template_base != actual_base || template_args.len() != actual_args.len() {
                return Ok(());
            }
            for (template_arg, actual_arg) in template_args.iter().zip(&actual_args) {
                infer_module_variant_substitution(
                    template_arg,
                    actual_arg,
                    generic_params,
                    substitutions,
                )?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn parse_module_named_type_application(name: &str) -> Option<(String, Vec<WaveType>)> {
    let (base, tail) = name.split_once('<')?;
    let inner = tail.strip_suffix('>')?;
    let arguments = split_top_level_generic_args(inner)?
        .into_iter()
        .map(|argument| {
            let token = parse_type(&argument)?;
            token_type_to_wave_type(&token)
        })
        .collect::<Option<Vec<_>>>()?;
    Some((base.trim().to_string(), arguments))
}

fn module_type_name(ty: &WaveType) -> String {
    match ty {
        WaveType::Infer => "infer".to_string(),
        WaveType::Int(bits) => format!("i{}", bits),
        WaveType::Uint(bits) => format!("u{}", bits),
        WaveType::Float(bits) => format!("f{}", bits),
        WaveType::Bool => "bool".to_string(),
        WaveType::Char => "char".to_string(),
        WaveType::Byte => "byte".to_string(),
        WaveType::String => "str".to_string(),
        WaveType::Pointer(inner) => format!("ptr<{}>", module_type_name(inner)),
        WaveType::Array(inner, size) => format!("array<{},{}>", module_type_name(inner), size),
        WaveType::Void => "void".to_string(),
        WaveType::Struct(name) | WaveType::Variant(name) => name.clone(),
    }
}

fn expression_literal_kind(expression: &Expression) -> Option<bool> {
    match expression {
        Expression::Literal(Literal::Int(_)) => Some(false),
        Expression::Literal(Literal::Float(_)) => Some(true),
        Expression::Grouped(inner) => expression_literal_kind(inner),
        _ => None,
    }
}

fn is_integer_type(ty: &WaveType) -> bool {
    matches!(
        ty,
        WaveType::Int(_) | WaveType::Uint(_) | WaveType::Byte | WaveType::Char
    )
}

fn substitute_callable_return(callable: &CallableType, type_args: &[WaveType]) -> WaveType {
    let substitutions = callable
        .generic_params
        .iter()
        .cloned()
        .zip(type_args.iter().cloned())
        .collect::<HashMap<_, _>>();
    substitute_type(&callable.return_type, &substitutions)
}

fn substitute_type(ty: &WaveType, substitutions: &HashMap<String, WaveType>) -> WaveType {
    match ty {
        WaveType::Struct(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        WaveType::Pointer(inner) => {
            WaveType::Pointer(Box::new(substitute_type(inner, substitutions)))
        }
        WaveType::Array(inner, size) => {
            WaveType::Array(Box::new(substitute_type(inner, substitutions)), *size)
        }
        WaveType::Variant(name) => WaveType::Variant(name.clone()),
        _ => ty.clone(),
    }
}

/// Removes the internal module hash prefix from names shown to Wave users.
pub fn demangle_module_names(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    const PREFIX: &str = "__wave_";
    while let Some(index) = rest.find(PREFIX) {
        out.push_str(&rest[..index]);
        let candidate = &rest[index + PREFIX.len()..];
        if candidate.len() >= 17
            && candidate.as_bytes()[..16]
                .iter()
                .all(|byte| byte.is_ascii_hexdigit())
            && candidate.as_bytes()[16] == b'_'
        {
            rest = &candidate[17..];
        } else {
            out.push_str(PREFIX);
            rest = candidate;
        }
    }
    out.push_str(rest);
    out
}

impl Resolver<'_> {
    fn resolve_module(
        &mut self,
        key: PathBuf,
        ast: Vec<ASTNode>,
        origin: usize,
        is_entry: bool,
    ) -> Result<ModuleInterface, WaveError> {
        if let Some(interface) = self.interfaces.get(&key) {
            return Ok(interface.clone());
        }
        if let Some(cycle_start) = self.visiting.iter().position(|path| path == &key) {
            let mut chain = self.visiting[cycle_start..]
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            chain.push(key.display().to_string());
            return Err(module_error(
                &key,
                "Import cycle",
                format!("import cycle detected: {}", chain.join(" -> ")),
                "remove one dependency edge or move shared declarations into a third module",
            ));
        }

        self.visiting.push(key.clone());
        let mut interface = collect_symbols(&key, &ast, is_entry)?;
        let mut names = NameContext {
            own: interface.symbols.clone(),
            ..NameContext::default()
        };

        for node in &ast {
            let ASTNode::Statement(StatementNode::Import(import)) = node else {
                continue;
            };

            let base_dir = key.parent().unwrap_or(Path::new("."));
            let mut loaded = HashSet::new();
            let unit =
                local_import_unit_with_config(&import.path, &mut loaded, base_dir, self.config)?;
            let child_key = canonical_key(&unit.abs_path);
            let child_origin = if let Some(index) = self.source_indices.get(&child_key) {
                *index
            } else {
                let index = self.sources.len();
                self.sources.push(ModuleSource {
                    path: unit.abs_path.clone(),
                    source: unit.source,
                });
                self.source_indices.insert(child_key.clone(), index);
                index
            };
            let child = self.resolve_module(child_key, unit.ast, child_origin, false)?;
            bind_import(&key, import, child.clone(), &mut names)?;
            if import.visibility == Visibility::Public {
                for selected in &import.selections {
                    let symbol = child
                        .symbols
                        .get(selected)
                        .expect("bind_import validated public selections")
                        .clone();
                    if interface.symbols.insert(selected.clone(), symbol).is_some() {
                        return Err(module_error(
                            &key,
                            "Duplicate public symbol",
                            format!("re-exported symbol '{}' conflicts in this module", selected),
                            "remove one re-export or rename the local declaration",
                        ));
                    }
                    let prefix = format!("{}::", selected);
                    for (qualified, constructor) in &child.symbols {
                        if qualified.starts_with(&prefix)
                            && constructor.kind == SymbolKind::VariantConstructor
                        {
                            interface
                                .symbols
                                .insert(qualified.clone(), constructor.clone());
                        }
                    }
                }
            }
        }

        let mut lowered = Vec::new();
        for node in ast {
            if matches!(node, ASTNode::Statement(StatementNode::Import(_))) {
                continue;
            }
            lowered.push(rewrite_top_level(node, &names, &key, is_entry)?);
        }

        self.visiting.pop();
        self.interfaces.insert(key, interface.clone());
        self.origins
            .extend(std::iter::repeat_n(origin, lowered.len()));
        self.ast.extend(lowered);
        Ok(interface)
    }
}

fn canonical_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn module_error(
    path: &Path,
    title: &str,
    message: impl Into<String>,
    help: impl Into<String>,
) -> WaveError {
    WaveError::new(
        WaveErrorKind::InvalidStatement(title.to_string()),
        message,
        path.display().to_string(),
        1,
        1,
    )
    .with_code("E3001")
    .with_context("module resolution")
    .with_help(help)
}

fn internal_name(path: &Path, name: &str, is_entry: bool) -> String {
    if is_entry {
        return name.to_string();
    }
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in path.to_string_lossy().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("__wave_{hash:016x}_{name}")
}

fn insert_symbol(
    path: &Path,
    symbols: &mut HashMap<String, ModuleSymbol>,
    source_name: &str,
    visibility: Visibility,
    kind: SymbolKind,
    is_entry: bool,
) -> Result<(), WaveError> {
    if symbols.contains_key(source_name) {
        // The semantic validator owns duplicate-declaration diagnostics because
        // it can point at the repeated source declaration. Both declarations
        // receive the same lowered name, so flattening cannot hide the error.
        return Ok(());
    }
    symbols.insert(
        source_name.to_string(),
        ModuleSymbol {
            lowered: internal_name(path, source_name, is_entry),
            visibility,
            kind,
        },
    );
    Ok(())
}

fn collect_symbols(
    path: &Path,
    ast: &[ASTNode],
    is_entry: bool,
) -> Result<ModuleInterface, WaveError> {
    let mut symbols = HashMap::new();
    for node in ast {
        match node {
            ASTNode::Function(function) => {
                if !is_entry && function.name == "main" {
                    return Err(module_error(
                        path,
                        "Invalid module entry point",
                        "function 'main' may only be declared in the entry module",
                        "remove `main` from the library module or rename it",
                    ));
                }
                insert_symbol(
                    path,
                    &mut symbols,
                    &function.name,
                    function.visibility,
                    SymbolKind::Function,
                    is_entry,
                )?;
            }
            ASTNode::ExternFunction(function) => insert_symbol(
                path,
                &mut symbols,
                &function.name,
                Visibility::Private,
                SymbolKind::Function,
                is_entry,
            )?,
            ASTNode::Struct(structure) => insert_symbol(
                path,
                &mut symbols,
                &structure.name,
                structure.visibility,
                SymbolKind::Struct,
                is_entry,
            )?,
            ASTNode::TypeAlias(alias) => insert_symbol(
                path,
                &mut symbols,
                &alias.name,
                alias.visibility,
                SymbolKind::Type,
                is_entry,
            )?,
            ASTNode::Enum(enumeration) => {
                insert_symbol(
                    path,
                    &mut symbols,
                    &enumeration.name,
                    enumeration.visibility,
                    SymbolKind::Type,
                    is_entry,
                )?;
                for variant in &enumeration.variants {
                    insert_symbol(
                        path,
                        &mut symbols,
                        &variant.name,
                        Visibility::Private,
                        SymbolKind::Value,
                        is_entry,
                    )?;
                    if enumeration.visibility == Visibility::Public {
                        symbols.insert(
                            format!("{}::{}", enumeration.name, variant.name),
                            ModuleSymbol {
                                lowered: internal_name(path, &variant.name, is_entry),
                                visibility: Visibility::Public,
                                kind: SymbolKind::Value,
                            },
                        );
                    }
                }
            }
            ASTNode::Variant(variant) => {
                insert_symbol(
                    path,
                    &mut symbols,
                    &variant.name,
                    variant.visibility,
                    SymbolKind::Type,
                    is_entry,
                )?;
                let lowered_owner = internal_name(path, &variant.name, is_entry);
                for case in &variant.cases {
                    symbols.insert(
                        format!("{}::{}", variant.name, case.name),
                        ModuleSymbol {
                            lowered: format!("{}::{}", lowered_owner, case.name),
                            visibility: variant.visibility,
                            kind: SymbolKind::VariantConstructor,
                        },
                    );
                }
            }
            ASTNode::Variable(variable)
                if matches!(variable.mutability, Mutability::Const | Mutability::Static) =>
            {
                insert_symbol(
                    path,
                    &mut symbols,
                    &variable.name,
                    variable.visibility,
                    SymbolKind::Value,
                    is_entry,
                )?;
            }
            _ => {}
        }
    }
    Ok(ModuleInterface { symbols })
}

fn default_namespace(path: &str) -> Option<String> {
    if path.starts_with("./") {
        return Path::new(path)
            .file_stem()
            .and_then(|name| name.to_str())
            .map(str::to_string);
    }
    Some(path.to_string())
}

fn bind_import(
    current_path: &Path,
    import: &ImportNode,
    interface: ModuleInterface,
    names: &mut NameContext,
) -> Result<(), WaveError> {
    if !import.selections.is_empty() {
        for selected in &import.selections {
            let Some(symbol) = interface.symbols.get(selected) else {
                return Err(module_error(
                    current_path,
                    "Unknown imported symbol",
                    format!("module '{}' has no symbol '{}'", import.path, selected),
                    "check the declaration name and the selected import list",
                ));
            };
            if symbol.visibility != Visibility::Public {
                return Err(module_error(
                    current_path,
                    "Private imported symbol",
                    format!(
                        "symbol '{}' is private in module '{}'",
                        selected, import.path
                    ),
                    "mark the declaration `pub` or remove it from the import list",
                ));
            }
            if names.own.contains_key(selected) {
                return Err(module_error(
                    current_path,
                    "Ambiguous imported symbol",
                    format!("imported symbol '{}' conflicts in this module", selected),
                    "rename the local declaration or use a qualified module import",
                ));
            }
            if let Some(existing) = names.selected.get(selected) {
                if existing == symbol {
                    continue;
                }
                return Err(module_error(
                    current_path,
                    "Ambiguous imported symbol",
                    format!("imported symbol '{}' conflicts in this module", selected),
                    "use a qualified module import to disambiguate the declarations",
                ));
            }
            names.selected.insert(selected.clone(), symbol.clone());
            let prefix = format!("{}::", selected);
            for (qualified, constructor) in &interface.symbols {
                if qualified.starts_with(&prefix)
                    && constructor.kind == SymbolKind::VariantConstructor
                {
                    names
                        .selected
                        .insert(qualified.clone(), constructor.clone());
                }
            }
        }
        return Ok(());
    }

    let namespace = import
        .alias
        .clone()
        .or_else(|| default_namespace(&import.path))
        .ok_or_else(|| {
            module_error(
                current_path,
                "Invalid import namespace",
                format!("cannot derive a namespace from import '{}'", import.path),
                "provide an explicit alias with `as name`",
            )
        })?;

    if let Some(existing) = names.namespaces.get(&namespace) {
        if existing == &interface {
            return Ok(());
        }
        return Err(module_error(
            current_path,
            "Duplicate import namespace",
            format!("namespace '{}' is bound to multiple modules", namespace),
            "use a distinct `as` alias for one import",
        ));
    }
    names.namespaces.insert(namespace, interface);
    Ok(())
}

fn resolve_name(
    name: &str,
    names: &NameContext,
    path: &Path,
) -> Result<Option<ModuleSymbol>, WaveError> {
    if name.contains("::") {
        if let Some(symbol) = names.own.get(name).or_else(|| names.selected.get(name)) {
            return Ok(Some(symbol.clone()));
        }
        let mut best: Option<(&str, &ModuleInterface)> = None;
        for (namespace, interface) in &names.namespaces {
            if name
                .strip_prefix(namespace)
                .is_some_and(|rest| rest.starts_with("::"))
                && best.is_none_or(|(current, _)| namespace.len() > current.len())
            {
                best = Some((namespace, interface));
            }
        }
        let Some((namespace, interface)) = best else {
            return Err(module_error(
                path,
                "Unknown import namespace",
                format!("qualified name '{}' uses an unknown module namespace", name),
                "import the module first or check its alias",
            ));
        };
        let symbol_name = &name[namespace.len() + 2..];
        let Some(symbol) = interface.symbols.get(symbol_name) else {
            return Err(module_error(
                path,
                "Unknown imported symbol",
                format!("module '{}' has no symbol '{}'", namespace, symbol_name),
                "check the public declaration name",
            ));
        };
        if symbol.visibility != Visibility::Public {
            return Err(module_error(
                path,
                "Private imported symbol",
                format!(
                    "symbol '{}' is private in module '{}'",
                    symbol_name, namespace
                ),
                "only `pub` declarations are accessible outside their module",
            ));
        }
        return Ok(Some(symbol.clone()));
    }

    Ok(names
        .own
        .get(name)
        .or_else(|| names.selected.get(name))
        .cloned())
}

fn rewrite_type(ty: WaveType, names: &NameContext, path: &Path) -> Result<WaveType, WaveError> {
    match ty {
        WaveType::Pointer(inner) => Ok(WaveType::Pointer(Box::new(rewrite_type(
            *inner, names, path,
        )?))),
        WaveType::Array(inner, size) => Ok(WaveType::Array(
            Box::new(rewrite_type(*inner, names, path)?),
            size,
        )),
        WaveType::Struct(name) => Ok(WaveType::Struct(rewrite_type_name(&name, names, path)?)),
        WaveType::Variant(name) => Ok(WaveType::Variant(rewrite_type_name(&name, names, path)?)),
        other => Ok(other),
    }
}

fn rewrite_type_name(name: &str, names: &NameContext, path: &Path) -> Result<String, WaveError> {
    if let Some((base, arguments)) = split_type_application(name) {
        let rewritten_base = rewrite_type_name(base, names, path)?;
        let rewritten_arguments = split_type_arguments(arguments)
            .into_iter()
            .map(|argument| rewrite_type_name(argument.trim(), names, path))
            .collect::<Result<Vec<_>, _>>()?;
        return Ok(format!(
            "{}<{}>",
            rewritten_base,
            rewritten_arguments.join(",")
        ));
    }

    match resolve_name(name, names, path)? {
        Some(symbol) if matches!(symbol.kind, SymbolKind::Struct | SymbolKind::Type) => {
            Ok(symbol.lowered)
        }
        Some(_) => Err(module_error(
            path,
            "Expected imported type",
            format!("symbol '{}' is not a type", name),
            "use a public struct, enum, or type alias in this position",
        )),
        None => Ok(name.to_string()),
    }
}

fn split_type_application(name: &str) -> Option<(&str, &str)> {
    let open = name.find('<')?;
    if !name.ends_with('>') || open == 0 {
        return None;
    }
    Some((&name[..open], &name[open + 1..name.len() - 1]))
}

fn split_type_arguments(arguments: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (index, ch) in arguments.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&arguments[start..index]);
                start = index + 1;
            }
            _ => {}
        }
    }
    out.push(&arguments[start..]);
    out
}

fn rewrite_top_level(
    node: ASTNode,
    names: &NameContext,
    path: &Path,
    is_entry: bool,
) -> Result<ASTNode, WaveError> {
    match node {
        ASTNode::Function(mut function) => {
            let original = function.name.clone();
            function.name = names.own[&original].lowered.clone();
            if !is_entry && original == "main" {
                function.name = internal_name(path, "main", false);
            }
            if !is_entry {
                if let Some(export) = &mut function.export {
                    if export.symbol.is_none() {
                        export.symbol = Some(original);
                    }
                }
            }
            rewrite_function(&mut function, names, path)?;
            Ok(ASTNode::Function(function))
        }
        ASTNode::ExternFunction(mut function) => {
            function.name = names.own[&function.name].lowered.clone();
            function.params = function
                .params
                .into_iter()
                .map(|(name, ty)| Ok((name, rewrite_type(ty, names, path)?)))
                .collect::<Result<_, WaveError>>()?;
            function.return_type = rewrite_type(function.return_type, names, path)?;
            Ok(ASTNode::ExternFunction(function))
        }
        ASTNode::Struct(mut structure) => {
            structure.name = names.own[&structure.name].lowered.clone();
            structure.fields = structure
                .fields
                .into_iter()
                .map(|(name, ty)| Ok((name, rewrite_type(ty, names, path)?)))
                .collect::<Result<_, WaveError>>()?;
            for method in &mut structure.methods {
                rewrite_function(method, names, path)?;
            }
            Ok(ASTNode::Struct(structure))
        }
        ASTNode::TypeAlias(mut alias) => {
            alias.name = names.own[&alias.name].lowered.clone();
            alias.target = rewrite_type(alias.target, names, path)?;
            Ok(ASTNode::TypeAlias(alias))
        }
        ASTNode::Enum(mut enumeration) => {
            enumeration.name = names.own[&enumeration.name].lowered.clone();
            enumeration.repr_type = rewrite_type(enumeration.repr_type, names, path)?;
            for variant in &mut enumeration.variants {
                variant.name = names.own[&variant.name].lowered.clone();
            }
            Ok(ASTNode::Enum(enumeration))
        }
        ASTNode::Variant(mut variant) => {
            variant.name = names.own[&variant.name].lowered.clone();
            for case in &mut variant.cases {
                case.payload_types = case
                    .payload_types
                    .drain(..)
                    .map(|ty| rewrite_type(ty, names, path))
                    .collect::<Result<_, _>>()?;
            }
            Ok(ASTNode::Variant(variant))
        }
        ASTNode::Variable(mut variable) => {
            variable.name = names.own[&variable.name].lowered.clone();
            variable.type_name = rewrite_type(variable.type_name, names, path)?;
            if let Some(value) = variable.initial_value.take() {
                variable.initial_value =
                    Some(rewrite_expression(value, names, path, &HashSet::new())?);
            }
            Ok(ASTNode::Variable(variable))
        }
        ASTNode::ProtoImpl(mut implementation) => {
            implementation.target = resolve_name(&implementation.target, names, path)?
                .map_or(implementation.target, |symbol| symbol.lowered);
            for method in &mut implementation.methods {
                rewrite_function(method, names, path)?;
            }
            Ok(ASTNode::ProtoImpl(implementation))
        }
        ASTNode::Statement(statement) => Ok(ASTNode::Statement(rewrite_statement(
            statement,
            names,
            path,
            &mut HashSet::new(),
        )?)),
        ASTNode::Expression(expression) => Ok(ASTNode::Expression(rewrite_expression(
            expression,
            names,
            path,
            &HashSet::new(),
        )?)),
        other => Ok(other),
    }
}

fn rewrite_function(
    function: &mut FunctionNode,
    names: &NameContext,
    path: &Path,
) -> Result<(), WaveError> {
    for parameter in &mut function.parameters {
        parameter.param_type = rewrite_type(parameter.param_type.clone(), names, path)?;
    }
    function.return_type = function
        .return_type
        .take()
        .map(|ty| rewrite_type(ty, names, path))
        .transpose()?;
    let mut locals = function
        .parameters
        .iter()
        .map(|parameter| parameter.name.clone())
        .collect::<HashSet<_>>();
    function.body = rewrite_block(std::mem::take(&mut function.body), names, path, &mut locals)?;
    Ok(())
}

fn rewrite_block(
    nodes: Vec<ASTNode>,
    names: &NameContext,
    path: &Path,
    locals: &mut HashSet<String>,
) -> Result<Vec<ASTNode>, WaveError> {
    let mut out = Vec::with_capacity(nodes.len());
    for node in nodes {
        match node {
            ASTNode::Variable(mut variable) => {
                variable.type_name = rewrite_type(variable.type_name, names, path)?;
                if let Some(value) = variable.initial_value.take() {
                    variable.initial_value = Some(rewrite_expression(value, names, path, locals)?);
                }
                locals.insert(variable.name.clone());
                out.push(ASTNode::Variable(variable));
            }
            ASTNode::Statement(statement) => out.push(ASTNode::Statement(rewrite_statement(
                statement, names, path, locals,
            )?)),
            ASTNode::Expression(expression) => out.push(ASTNode::Expression(rewrite_expression(
                expression, names, path, locals,
            )?)),
            other => out.push(other),
        }
    }
    Ok(out)
}

fn rewrite_statement(
    statement: StatementNode,
    names: &NameContext,
    path: &Path,
    locals: &mut HashSet<String>,
) -> Result<StatementNode, WaveError> {
    Ok(match statement {
        StatementNode::PrintFormat { format, args } => StatementNode::PrintFormat {
            format,
            args: rewrite_expressions(args, names, path, locals)?,
        },
        StatementNode::PrintlnFormat { format, args } => StatementNode::PrintlnFormat {
            format,
            args: rewrite_expressions(args, names, path, locals)?,
        },
        StatementNode::Input { format, args } => StatementNode::Input {
            format,
            args: rewrite_expressions(args, names, path, locals)?,
        },
        StatementNode::If {
            condition,
            body,
            else_if_blocks,
            else_block,
        } => {
            let condition = rewrite_expression(condition, names, path, locals)?;
            let mut body_scope = locals.clone();
            let body = rewrite_block(body, names, path, &mut body_scope)?;
            let else_if_blocks = else_if_blocks
                .map(|blocks| {
                    blocks
                        .into_iter()
                        .map(|(condition, body)| {
                            let condition = rewrite_expression(condition, names, path, locals)?;
                            let mut scope = locals.clone();
                            Ok((condition, rewrite_block(body, names, path, &mut scope)?))
                        })
                        .collect::<Result<Vec<_>, WaveError>>()
                        .map(Box::new)
                })
                .transpose()?;
            let else_block = else_block
                .map(|body| {
                    let mut scope = locals.clone();
                    rewrite_block(*body, names, path, &mut scope).map(Box::new)
                })
                .transpose()?;
            StatementNode::If {
                condition,
                body,
                else_if_blocks,
                else_block,
            }
        }
        StatementNode::For {
            initialization,
            condition,
            increment,
            body,
        } => {
            let mut scope = locals.clone();
            let initialization = rewrite_block(vec![*initialization], names, path, &mut scope)?
                .into_iter()
                .next()
                .expect("for initializer is preserved");
            let condition = rewrite_expression(condition, names, path, &scope)?;
            let increment = rewrite_expression(increment, names, path, &scope)?;
            let body = rewrite_block(body, names, path, &mut scope)?;
            StatementNode::For {
                initialization: Box::new(initialization),
                condition,
                increment,
                body,
            }
        }
        StatementNode::While { condition, body } => {
            let condition = rewrite_expression(condition, names, path, locals)?;
            let mut scope = locals.clone();
            let body = rewrite_block(body, names, path, &mut scope)?;
            StatementNode::While { condition, body }
        }
        StatementNode::Match { value, arms } => StatementNode::Match {
            value: rewrite_expression(value, names, path, locals)?,
            arms: arms
                .into_iter()
                .map(|mut arm| {
                    rewrite_match_pattern(&mut arm.pattern, names, path, locals)?;
                    let mut scope = locals.clone();
                    collect_pattern_bindings(&arm.pattern, &mut scope);
                    arm.body = rewrite_block(arm.body, names, path, &mut scope)?;
                    Ok(arm)
                })
                .collect::<Result<_, WaveError>>()?,
        },
        StatementNode::Assign { variable, value } => StatementNode::Assign {
            variable: if locals.contains(&variable) {
                variable
            } else {
                resolve_name(&variable, names, path)?.map_or(variable, |symbol| symbol.lowered)
            },
            value: rewrite_expression(value, names, path, locals)?,
        },
        StatementNode::AsmBlock {
            instructions,
            inputs,
            outputs,
            clobbers,
        } => StatementNode::AsmBlock {
            instructions,
            inputs: inputs
                .into_iter()
                .map(|(constraint, expression)| {
                    Ok((
                        constraint,
                        rewrite_expression(expression, names, path, locals)?,
                    ))
                })
                .collect::<Result<_, WaveError>>()?,
            outputs: outputs
                .into_iter()
                .map(|(constraint, expression)| {
                    Ok((
                        constraint,
                        rewrite_expression(expression, names, path, locals)?,
                    ))
                })
                .collect::<Result<_, WaveError>>()?,
            clobbers,
        },
        StatementNode::Return(value) => StatementNode::Return(
            value
                .map(|expression| rewrite_expression(expression, names, path, locals))
                .transpose()?,
        ),
        StatementNode::Expression(expression) => {
            StatementNode::Expression(rewrite_expression(expression, names, path, locals)?)
        }
        other => other,
    })
}

fn rewrite_expressions(
    expressions: Vec<Expression>,
    names: &NameContext,
    path: &Path,
    locals: &HashSet<String>,
) -> Result<Vec<Expression>, WaveError> {
    expressions
        .into_iter()
        .map(|expression| rewrite_expression(expression, names, path, locals))
        .collect()
}

fn rewrite_match_pattern(
    pattern: &mut MatchPattern,
    names: &NameContext,
    path: &Path,
    locals: &HashSet<String>,
) -> Result<(), WaveError> {
    match pattern {
        MatchPattern::Ident(name) => {
            if !locals.contains(name) {
                if let Some(symbol) = resolve_name(name, names, path)? {
                    *name = symbol.lowered;
                }
            }
        }
        MatchPattern::Variant {
            variant_type,
            case_name,
            payloads,
        } => {
            let qualified = format!("{}::{}", variant_type, case_name);
            let symbol = resolve_name(&qualified, names, path)?.ok_or_else(|| {
                module_error(
                    path,
                    "Unknown variant case",
                    format!("variant case '{}' is not declared", qualified),
                    "check the variant type and case name",
                )
            })?;
            if symbol.kind != SymbolKind::VariantConstructor {
                return Err(module_error(
                    path,
                    "Invalid variant pattern",
                    format!("'{}' is not a variant case", qualified),
                    "use a qualified case declared by a variant",
                ));
            }
            let (owner, case) = symbol.lowered.rsplit_once("::").unwrap();
            *variant_type = owner.to_string();
            *case_name = case.to_string();
            for payload in payloads {
                rewrite_match_pattern(payload, names, path, locals)?;
            }
        }
        MatchPattern::Int(_) | MatchPattern::Binding(_) | MatchPattern::Wildcard => {}
    }
    Ok(())
}

fn collect_pattern_bindings(pattern: &MatchPattern, locals: &mut HashSet<String>) {
    match pattern {
        MatchPattern::Binding(name) => {
            locals.insert(name.clone());
        }
        MatchPattern::Variant { payloads, .. } => {
            for payload in payloads {
                collect_pattern_bindings(payload, locals);
            }
        }
        MatchPattern::Int(_) | MatchPattern::Ident(_) | MatchPattern::Wildcard => {}
    }
}

fn rewrite_expression(
    expression: Expression,
    names: &NameContext,
    path: &Path,
    locals: &HashSet<String>,
) -> Result<Expression, WaveError> {
    Ok(match expression {
        Expression::StructLiteral { name, fields } => Expression::StructLiteral {
            name: resolve_name(&name, names, path)?.map_or(name, |symbol| symbol.lowered),
            fields: fields
                .into_iter()
                .map(|(name, value)| Ok((name, rewrite_expression(value, names, path, locals)?)))
                .collect::<Result<_, WaveError>>()?,
        },
        Expression::FunctionCall {
            name,
            type_args,
            args,
        } => {
            let symbol = resolve_name(&name, names, path)?;
            let type_args = type_args
                .into_iter()
                .map(|ty| rewrite_type(ty, names, path))
                .collect::<Result<Vec<_>, _>>()?;
            let args = rewrite_expressions(args, names, path, locals)?;
            match symbol {
                Some(symbol) if symbol.kind == SymbolKind::Struct && type_args.is_empty() => {
                    if !args.is_empty() {
                        return Err(module_error(
                            path,
                            "Invalid struct constructor",
                            format!("struct '{}' must be initialized with named fields", name),
                            "use `Type { field: value }`; `Type()` is only valid for empty structs",
                        ));
                    }
                    Expression::StructLiteral {
                        name: symbol.lowered,
                        fields: Vec::new(),
                    }
                }
                Some(symbol)
                    if matches!(
                        symbol.kind,
                        SymbolKind::Function | SymbolKind::VariantConstructor
                    ) =>
                {
                    Expression::FunctionCall {
                        name: symbol.lowered,
                        type_args,
                        args,
                    }
                }
                Some(_) => {
                    return Err(module_error(
                        path,
                        "Symbol is not callable",
                        format!("symbol '{}' cannot be called", name),
                        "call a function or construct an empty struct",
                    ))
                }
                None => Expression::FunctionCall {
                    name,
                    type_args,
                    args,
                },
            }
        }
        Expression::MethodCall { object, name, args } => Expression::MethodCall {
            object: Box::new(rewrite_expression(*object, names, path, locals)?),
            name,
            args: rewrite_expressions(args, names, path, locals)?,
        },
        Expression::Variable(name) => Expression::Variable(if locals.contains(&name) {
            name
        } else {
            resolve_name(&name, names, path)?.map_or(name, |symbol| symbol.lowered)
        }),
        Expression::Deref(inner) => {
            Expression::Deref(Box::new(rewrite_expression(*inner, names, path, locals)?))
        }
        Expression::AddressOf(inner) => {
            Expression::AddressOf(Box::new(rewrite_expression(*inner, names, path, locals)?))
        }
        Expression::BinaryExpression {
            left,
            operator,
            right,
        } => Expression::BinaryExpression {
            left: Box::new(rewrite_expression(*left, names, path, locals)?),
            operator,
            right: Box::new(rewrite_expression(*right, names, path, locals)?),
        },
        Expression::IndexAccess { target, index } => Expression::IndexAccess {
            target: Box::new(rewrite_expression(*target, names, path, locals)?),
            index: Box::new(rewrite_expression(*index, names, path, locals)?),
        },
        Expression::ArrayLiteral(values) => {
            Expression::ArrayLiteral(rewrite_expressions(values, names, path, locals)?)
        }
        Expression::Grouped(inner) => {
            Expression::Grouped(Box::new(rewrite_expression(*inner, names, path, locals)?))
        }
        Expression::AssignOperation {
            target,
            operator,
            value,
        } => Expression::AssignOperation {
            target: Box::new(rewrite_expression(*target, names, path, locals)?),
            operator,
            value: Box::new(rewrite_expression(*value, names, path, locals)?),
        },
        Expression::Assignment { target, value } => Expression::Assignment {
            target: Box::new(rewrite_expression(*target, names, path, locals)?),
            value: Box::new(rewrite_expression(*value, names, path, locals)?),
        },
        Expression::AsmBlock {
            instructions,
            inputs,
            outputs,
            clobbers,
        } => Expression::AsmBlock {
            instructions,
            inputs: inputs
                .into_iter()
                .map(|(constraint, expression)| {
                    Ok((
                        constraint,
                        rewrite_expression(expression, names, path, locals)?,
                    ))
                })
                .collect::<Result<_, WaveError>>()?,
            outputs: outputs
                .into_iter()
                .map(|(constraint, expression)| {
                    Ok((
                        constraint,
                        rewrite_expression(expression, names, path, locals)?,
                    ))
                })
                .collect::<Result<_, WaveError>>()?,
            clobbers,
        },
        Expression::FieldAccess { object, field } => Expression::FieldAccess {
            object: Box::new(rewrite_expression(*object, names, path, locals)?),
            field,
        },
        Expression::Unary { operator, expr } => Expression::Unary {
            operator,
            expr: Box::new(rewrite_expression(*expr, names, path, locals)?),
        },
        Expression::Cast { expr, target_type } => Expression::Cast {
            expr: Box::new(rewrite_expression(*expr, names, path, locals)?),
            target_type: rewrite_type(target_type, names, path)?,
        },
        Expression::IncDec { kind, target } => Expression::IncDec {
            kind,
            target: Box::new(rewrite_expression(*target, names, path, locals)?),
        },
        other => other,
    })
}
