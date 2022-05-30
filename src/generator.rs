use crate::parser::{ArrayInfo, Expr, TypeName, Value, ValueType};
use std::borrow::Cow;

#[derive(Debug)]
pub enum ExprType {
    Const(String),
    Variable(String),
}

pub fn gen_msg<'a>(type_name: &'a str, exprs: &[Expr]) -> Vec<Cow<'a, str>> {
    let mut lines = vec!["use safe_drive::msgs::*;".into(), "".into()];

    let mut const_val = Vec::new();
    let mut variables = Vec::new();

    for expr in exprs.iter() {
        match gen_expr(expr) {
            ExprType::Const(val) => const_val.push(val),
            ExprType::Variable(val) => variables.push(val),
        }
    }

    for c in const_val {
        lines.push(c.into());
    }

    lines.push("".into());
    lines.push("#[repr(C)]".into());
    lines.push(format!("struct {type_name} {{").into());

    for v in variables {
        lines.push(v.into());
    }

    lines.push("}".into());

    lines
}

pub fn gen_expr(expr: &Expr) -> ExprType {
    match expr {
        Expr::Variable {
            type_name,
            var_name,
            value,
            comment,
        } => match value {
            Some(ValueType::Const(val)) => {
                let ty = gen_const_type(type_name);
                let v = gen_value(val);
                let result = if let Some(c) = comment {
                    format!("const {var_name}: {ty} = {v}; //{c}",)
                } else {
                    format!("const {var_name}: {ty} = {v};",)
                };
                ExprType::Const(result)
            }
            _ => {
                let ty = gen_type(type_name);
                let result = if let Some(c) = comment {
                    format!("    {var_name}: {ty}, //{c}")
                } else {
                    format!("    {var_name}: {ty},")
                };
                ExprType::Variable(result)
            }
        },
        _ => unreachable!(),
    }
}

fn gen_value(value: &Value) -> String {
    format!("{value}")
}

fn gen_type(type_name: &'_ TypeName) -> Cow<'_, str> {
    match type_name {
        TypeName::Type {
            type_name,
            array_info,
        } => {
            let type_str = if let Some(prim) = gen_primitives(type_name) {
                prim
            } else {
                type_name
            };
            gen_array_type(type_str, array_info)
        }
        TypeName::String(array_info) => {
            let type_str = "rosidl_runtime_c__String";
            gen_array_type(type_str, array_info)
        }
        TypeName::LimitedString { size, array_info } => todo!(),
        TypeName::ScopedType {
            scope,
            type_name,
            array_info,
        } => todo!(),
    }
}

fn gen_array_type<'a>(type_str: &'a str, array_info: &ArrayInfo) -> Cow<'a, str> {
    match array_info {
        ArrayInfo::Dynamic => todo!(),
        ArrayInfo::Limited(n) => todo!(),
        ArrayInfo::Static(n) => format!("[{type_str}; {n}]").into(),
        ArrayInfo::NotArray => type_str.into(),
    }
}

fn gen_primitives(type_name: &str) -> Option<&str> {
    let t = match type_name {
        "bool" => "bool",
        "int8" => "i8",
        "uint8" => "u8",
        "int16" => "i16",
        "uint16" => "u16",
        "int32" => "i32",
        "uint32" => "u32",
        "int64" => "i64",
        "uint64" => "u64",
        "float32" => "f32",
        "float64" => "f64",
        _ => return None,
    };
    Some(t)
}

fn gen_const_type(type_name: &'_ TypeName) -> Cow<'_, str> {
    if let TypeName::String(array_info) = type_name {
        gen_array_type("&[u8]", array_info)
    } else {
        gen_type(type_name)
    }
}
