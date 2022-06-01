use crate::parser::{ArrayInfo, Expr, TypeName, Value, ValueType};
use std::borrow::Cow;

#[derive(Debug)]
pub enum ExprType {
    Const(String),
    Variable(String),
}

pub fn gen_msg<'a>(module_name: &str, type_name: &'a str, exprs: &[Expr]) -> Vec<Cow<'a, str>> {
    let mut lines = vec!["use safe_drive::msgs::*;".into()];

    let cfun = format!(
        "
extern \"C\" {{
    fn {module_name}__msg__{type_name}__init(msg: *mut {type_name}) -> bool;
    fn {module_name}__msg__{type_name}__fini(msg: *mut {type_name});
    fn {module_name}__msg__{type_name}__Sequence__init(msg: *mut {type_name}Sequence, size: usize) -> bool;
    fn {module_name}__msg__{type_name}__Sequence__fini(msg: *mut {type_name}Sequence);
}}
"
    );
    lines.push(cfun.into());

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

    // generate struct
    lines.push("".into());
    lines.push("#[repr(C)]".into());
    lines.push("#[derive(Debug)]".into());
    lines.push(format!("pub struct {type_name} {{").into());

    for v in variables {
        lines.push(v.into());
    }

    lines.push("}".into());

    // generate impl and struct of sequence
    let impl_str = format!(
        "
impl {type_name} {{
    pub fn new() -> Option<Self> {{
        let mut msg: Self = unsafe {{ std::mem::MaybeUninit::zeroed().assume_init() }};
        if unsafe {{ {module_name}__msg__{type_name}__init(&mut self) }} {{
            Some(msg)
        }} else {{
            None
        }}
    }}
}}

impl Drop for {type_name} {{
    fn drop(&mut self) {{
        unsafe {{ {module_name}__msg__{type_name}__fini(&mut self) }};
    }}
}}

#[repr(C)]
#[derive(Debug)]
pub struct {type_name}Sequence {{
    data: *mut {type_name},
    size: usize,
    capacity: usize,
}}

impl {type_name}Sequence {{
    pub fn new(size: usize) -> Option<Self> {{
        let mut msg: Self = unsafe {{ std::mem::MaybeUninit::zeroed().assume_init() }};
        if unsafe {{ {module_name}__msg__{type_name}__Sequence__init(&mut self, size) }} {{
            Some(msg)
        }} else {{
            None
        }}
    }}
}}

impl Drop for {type_name}Sequence {{
    fn drop(&mut self) {{
        unsafe {{ {module_name}__msg__{type_name}__Sequence__fini(&mut self) }};
    }}
}}
"
    );

    lines.push(impl_str.into());

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
                    format!("pub const {var_name}: {ty} = {v}; //{c}",)
                } else {
                    format!("pub const {var_name}: {ty} = {v};",)
                };
                ExprType::Const(result)
            }
            _ => {
                let ty = gen_type(type_name);
                let result = if let Some(c) = comment {
                    format!("    pub {var_name}: {ty}, //{c}")
                } else {
                    format!("    pub {var_name}: {ty},")
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
                prim.to_string()
            } else {
                format!("super::{type_name}")
            };
            gen_array_type(type_str.into(), array_info)
        }
        TypeName::String(array_info) => {
            let type_str = "rosidl_runtime_c__String";
            gen_array_type(type_str.into(), array_info)
        }
        TypeName::LimitedString {
            size: _size,
            array_info,
        } => {
            let type_str = "rosidl_runtime_c__String";
            gen_array_type(type_str.into(), array_info)
        }
        TypeName::ScopedType {
            scope,
            type_name,
            array_info,
        } => {
            let type_str = match scope.as_ref() {
                "std_msgs" => {
                    format!("{scope}__msg__{type_name}")
                }
                _ => {
                    format!("{scope}::msg::{type_name}")
                }
            };

            let arr = gen_array_type(type_str.into(), array_info);
            arr.into_owned().into()
        }
    }
}

fn gen_array_type<'a>(type_str: Cow<'a, str>, array_info: &ArrayInfo) -> Cow<'a, str> {
    match array_info {
        ArrayInfo::Dynamic => gen_seq_type(type_str).into(),
        ArrayInfo::Limited(_n) => gen_seq_type(type_str).into(),
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
        gen_array_type("&[u8]".into(), array_info)
    } else {
        gen_type(type_name)
    }
}

fn gen_seq_type(type_str: Cow<'_, str>) -> Cow<'_, str> {
    match type_str.as_ref() {
        "bool" => "rosidl_runtime_c__bool__Sequence".into(),
        "i8" => "rosidl_runtime_c__int8__Sequence".into(),
        "i16" => "rosidl_runtime_c__int16__Sequence".into(),
        "i32" => "rosidl_runtime_c__int32__Sequence".into(),
        "i64" => "rosidl_runtime_c__int64__Sequence".into(),
        "u8" => "rosidl_runtime_c__uint8__Sequence".into(),
        "u16" => "rosidl_runtime_c__uint16__Sequence".into(),
        "u32" => "rosidl_runtime_c__uint32__Sequence".into(),
        "u64" => "rosidl_runtime_c__uint64__Sequence".into(),
        "f32" => "rosidl_runtime_c__float32__Sequence".into(),
        "f64" => "rosidl_runtime_c__float64__Sequence".into(),
        "rosidl_runtime_c__String" => "rosidl_runtime_c__string__Sequence".into(),
        _ => format!("{type_str}Sequence").into(),
    }
}
