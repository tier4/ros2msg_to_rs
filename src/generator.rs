use crate::parser::{ArrayInfo, Expr, TypeName, Value, ValueType};
use std::{
    borrow::Cow,
    collections::{BTreeSet, VecDeque},
};

#[derive(Default)]
pub struct Generator {
    pub libs: BTreeSet<String>,
    lib_name: String,
    safe_drive_path: String,
    disable_common_interfaces: bool,
}

#[derive(Debug)]
pub enum ExprType {
    Const(String),
    Variable(String),
}

impl Generator {
    pub fn new(lib_name: String, safe_drive_path: String, disable_common_interfaces: bool) -> Self {
        Self {
            libs: Default::default(),
            lib_name,
            safe_drive_path,
            disable_common_interfaces,
        }
    }

    pub fn gen_srv<'a>(
        &mut self,
        module_name: &str,
        type_name: &'a str,
        exprs_req: &[Expr],
        exprs_resp: &[Expr],
    ) -> VecDeque<Cow<'a, str>> {
        let mut lines = VecDeque::new();
        lines.push_back("use super::super::*;".into());
        lines.push_back("use super::super::super::*;".into());
        lines.push_back(format!("use {}::msg::*;", self.safe_drive_path).into());
        lines.push_back(format!("use {}::rcl;", self.safe_drive_path).into());
        lines.push_back(format!("use {}::msg::common_interfaces::*;", self.safe_drive_path).into());

        let mut const_val = Vec::new();
        let mut var_req = Vec::new();
        let mut var_resp = Vec::new();

        for expr in exprs_req.iter() {
            match self.gen_expr(expr, type_name) {
                ExprType::Const(val) => const_val.push(val),
                ExprType::Variable(val) => var_req.push(val),
            }
        }

        for expr in exprs_resp.iter() {
            match self.gen_expr(expr, type_name) {
                ExprType::Const(val) => const_val.push(val),
                ExprType::Variable(val) => var_resp.push(val),
            }
        }

        // generate constant values
        for c in const_val {
            lines.push_back(c.into());
        }

        // generate C functions
        gen_cfun_srv(&mut lines, module_name, type_name);

        // generate struct of request
        lines.push_back("".into());
        lines.push_back("#[repr(C)]".into());
        lines.push_back("#[derive(Debug)]".into());
        lines.push_back(format!("pub struct {type_name}Request {{").into());

        if var_req.is_empty() {
            lines.push_back("    _unused: u8".into());
        } else {
            for v in var_req {
                lines.push_back(v.into());
            }
        }

        lines.push_back("}".into());

        // generate struct of response
        lines.push_back("".into());
        lines.push_back("#[repr(C)]".into());
        lines.push_back("#[derive(Debug)]".into());
        lines.push_back(format!("pub struct {type_name}Response {{").into());

        if var_resp.is_empty() {
            lines.push_back("    _unused: u8".into());
        } else {
            for v in var_resp {
                lines.push_back(v.into());
            }
        }

        lines.push_back("}".into());

        // generate impl {type_name}(Request|Response) and struct {type_name}(Request|Response)Sequence
        gen_impl_and_seq_srv(&mut lines, module_name, type_name);

        lines.push_front("// This file was automatically generated by ros2msg_to_rs (https://github.com/tier4/ros2msg_to_rs).".into());

        lines
    }

    pub fn gen_msg<'a>(
        &mut self,
        module_name: &str,
        type_name: &'a str,
        exprs: &[Expr],
    ) -> VecDeque<Cow<'a, str>> {
        let mut lines = VecDeque::new();
        lines.push_back("use super::*;".into());
        lines.push_back("use super::super::super::*;".into());
        lines.push_back(format!("use {}::msg::*;", self.safe_drive_path).into());
        lines.push_back(format!("use {}::rcl;", self.safe_drive_path).into());

        if !self.disable_common_interfaces {
            lines.push_back(
                format!("use {}::msg::common_interfaces::*;", self.safe_drive_path).into(),
            );
        }

        let mut const_val = Vec::new();
        let mut variables = Vec::new();

        for expr in exprs.iter() {
            match self.gen_expr(expr, type_name) {
                ExprType::Const(val) => const_val.push(val),
                ExprType::Variable(val) => variables.push(val),
            }
        }

        // generate constant values
        for c in const_val {
            lines.push_back(c.into());
        }

        // generate C functions
        gen_cfun_msg(&mut lines, module_name, type_name);

        // generate struct
        lines.push_back("".into());
        lines.push_back("#[repr(C)]".into());
        lines.push_back("#[derive(Debug)]".into());
        lines.push_back(format!("pub struct {type_name} {{").into());

        if variables.is_empty() {
            lines.push_back("    _unused: u8".into());
        } else {
            for v in variables {
                lines.push_back(v.into());
            }
        }

        lines.push_back("}".into());

        // generate impl {type_name} and struct {type_name}Sequence
        gen_impl_and_seq_msg(&mut lines, module_name, type_name);

        lines.push_front("// This file was automatically generated by ros2msg_to_rs (https://github.com/tier4/ros2msg_to_rs).".into());

        lines
    }

    fn gen_expr(&mut self, expr: &Expr, msg_type_name: &str) -> ExprType {
        match expr {
            Expr::Variable {
                type_name,
                var_name,
                value,
                comment,
            } => {
                let var_name = crate::mangle(var_name.as_str());
                match value {
                    Some(ValueType::Const(val)) => {
                        let ty = self.gen_const_type(type_name, msg_type_name);
                        let v = gen_value(val);
                        let result = if let Some(c) = comment {
                            format!("pub const {var_name}: {ty} = {v}; //{c}",)
                        } else {
                            format!("pub const {var_name}: {ty} = {v};",)
                        };
                        ExprType::Const(result)
                    }
                    _ => {
                        let ty = self.gen_type(type_name, msg_type_name);
                        let result = if let Some(c) = comment {
                            format!("    pub {var_name}: {ty}, //{c}")
                        } else {
                            format!("    pub {var_name}: {ty},")
                        };
                        ExprType::Variable(result)
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn gen_type(&mut self, type_name: &'_ TypeName, msg_type_name: &str) -> Cow<'_, str> {
        match type_name {
            TypeName::Type {
                type_name,
                array_info,
            } => {
                let type_str = if let Some(prim) = gen_primitives(type_name) {
                    prim.to_string()
                } else {
                    format!("{type_name}")
                };
                self.gen_array_type(None, type_str.into(), array_info, msg_type_name)
            }
            TypeName::String(array_info) => {
                let type_str = format!("{}::msg::RosString<0>", self.safe_drive_path);
                self.gen_string_array_type(type_str.into(), 0, array_info)
            }
            TypeName::LimitedString { size, array_info } => {
                let type_str = format!("{}::msg::RosString<{size}>", self.safe_drive_path);
                self.gen_string_array_type(type_str.into(), *size, array_info)
            }
            TypeName::ScopedType {
                scope,
                type_name,
                array_info,
            } => {
                let type_str = if self.lib_name == *scope {
                    type_name.clone()
                } else {
                    match scope.as_ref() {
                        "builtin_interfaces" => {
                            println!(
                                "Warning: {}::{msg_type_name} uses builtin_interfaces::{type_name} which causes the year-2038 problem.",
                                self.lib_name
                            );
                            match type_name.as_ref() {
                                "Time" => "builtin_interfaces::UnsafeTime".into(),
                                "Duration" => "builtin_interfaces::UnsafeDuration".into(),
                                _ => panic!("unsupported type: builtin_interfaces::{type_name}"),
                            }
                        }
                        _ => {
                            self.libs.insert(scope.clone());
                            format!("{scope}::msg::{type_name}")
                        }
                    }
                };

                let arr = self.gen_array_type(
                    Some(scope.as_str()),
                    type_str.into(),
                    array_info,
                    msg_type_name,
                );
                arr.into_owned().into()
            }
        }
    }

    fn gen_array_type<'a>(
        &mut self,
        scope: Option<&str>,
        type_str: Cow<'a, str>,
        array_info: &ArrayInfo,
        type_name: &str,
    ) -> Cow<'a, str> {
        match array_info {
            ArrayInfo::Dynamic => self.gen_seq_type(scope, type_str, 0, type_name).into(),
            ArrayInfo::Limited(n) => self.gen_seq_type(scope, type_str, *n, type_name).into(),
            ArrayInfo::Static(n) => format!("[{type_str}; {n}]").into(),
            ArrayInfo::NotArray => type_str.into(),
        }
    }

    fn gen_string_array_type<'a>(
        &mut self,
        type_str: Cow<'a, str>,
        strlen: usize,
        array_info: &ArrayInfo,
    ) -> Cow<'a, str> {
        match array_info {
            ArrayInfo::Dynamic => {
                format!("{}::msg::RosStringSeq<{strlen}, 0>", self.safe_drive_path).into()
            }
            ArrayInfo::Limited(n) => {
                format!("{}::msg::RosStringSeq<{strlen}, {n}>", self.safe_drive_path).into()
            }
            ArrayInfo::Static(n) => format!("[{type_str}; {n}]").into(),
            ArrayInfo::NotArray => type_str.into(),
        }
    }

    fn gen_const_type(&mut self, type_name: &'_ TypeName, msg_type_name: &str) -> Cow<'_, str> {
        if let TypeName::String(array_info) = type_name {
            self.gen_array_type(None, "&[u8]".into(), array_info, msg_type_name)
        } else {
            self.gen_type(type_name, msg_type_name)
        }
    }

    fn gen_seq_type<'a>(
        &self,
        scope: Option<&str>,
        type_str: Cow<'a, str>,
        size: usize,
        type_name: &str,
    ) -> Cow<'a, str> {
        match type_str.as_ref() {
            "bool" => format!("{}::msg::BoolSeq<{size}>", self.safe_drive_path).into(),
            "i8" => format!("{}::msg::I8Seq<{size}>", self.safe_drive_path).into(),
            "i16" => format!("{}::msg::I16Seq<{size}>", self.safe_drive_path).into(),
            "i32" => format!("{}::msg::I32Seq<{size}>", self.safe_drive_path).into(),
            "i64" => format!("{}::msg::I64Seq<{size}>", self.safe_drive_path).into(),
            "u8" => format!("{}::msg::U8Seq<{size}>", self.safe_drive_path).into(),
            "u16" => format!("{}::msg::U16Seq<{size}>", self.safe_drive_path).into(),
            "u32" => format!("{}::msg::U32Seq<{size}>", self.safe_drive_path).into(),
            "u64" => format!("{}::msg::U64Seq<{size}>", self.safe_drive_path).into(),
            "f32" => format!("{}::msg::F32Seq<{size}>", self.safe_drive_path).into(),
            "f64" => format!("{}::msg::F64Seq<{size}>", self.safe_drive_path).into(),
            _ => match scope {
                Some("builtin_interfaces") => {
                    println!(
                        "Warning: {}::{type_name} uses builtin_interfaces::{type_str} which causes the year-2038 problem.",
                        self.lib_name
                    );

                    match type_str.as_ref() {
                        "Time" => format!(
                            "{}::msg::builtin_interfaces::UnsafeTimeSeq<{size}>",
                            self.safe_drive_path
                        )
                        .into(),
                        "Duration" => format!(
                            "{}::msg::builtin_interfaces::UnsafeDurationSeq<{size}>",
                            self.safe_drive_path
                        )
                        .into(),
                        _ => panic!("unsupported type: builtin_interfaces::{type_str}"),
                    }
                }
                _ => format!("{type_str}Seq<{size}>").into(),
            },
        }
    }
}

fn gen_value(value: &Value) -> String {
    format!("{value}")
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
        "byte" => "u8",
        "char" => "i8",
        _ => return None,
    };
    Some(t)
}

fn gen_cfun_msg(lines: &mut VecDeque<Cow<'_, str>>, module_name: &str, type_name: &str) {
    let cfun = format!(
        "
extern \"C\" {{
    fn {module_name}__msg__{type_name}__init(msg: *mut {type_name}) -> bool;
    fn {module_name}__msg__{type_name}__fini(msg: *mut {type_name});
    fn {module_name}__msg__{type_name}__are_equal(lhs: *const {type_name}, rhs: *const {type_name}) -> bool;
    fn {module_name}__msg__{type_name}__Sequence__init(msg: *mut {type_name}SeqRaw, size: usize) -> bool;
    fn {module_name}__msg__{type_name}__Sequence__fini(msg: *mut {type_name}SeqRaw);
    fn {module_name}__msg__{type_name}__Sequence__are_equal(lhs: *const {type_name}SeqRaw, rhs: *const {type_name}SeqRaw) -> bool;
    fn rosidl_typesupport_c__get_message_type_support_handle__{module_name}__msg__{type_name}() -> *const rcl::rosidl_message_type_support_t;
}}
"
    );
    lines.push_back(cfun.into());
}

fn gen_cfun_srv(lines: &mut VecDeque<Cow<'_, str>>, module_name: &str, type_name: &str) {
    let cfun = format!(
        "
extern \"C\" {{
    fn {module_name}__srv__{type_name}_Request__init(msg: *mut {type_name}Request) -> bool;
    fn {module_name}__srv__{type_name}_Request__fini(msg: *mut {type_name}Request);
    fn {module_name}__srv__{type_name}_Request__Sequence__init(msg: *mut {type_name}RequestSeqRaw, size: usize) -> bool;
    fn {module_name}__srv__{type_name}_Request__Sequence__fini(msg: *mut {type_name}RequestSeqRaw);
    fn {module_name}__srv__{type_name}_Response__init(msg: *mut {type_name}Response) -> bool;
    fn {module_name}__srv__{type_name}_Response__fini(msg: *mut {type_name}Response);
    fn {module_name}__srv__{type_name}_Response__Sequence__init(msg: *mut {type_name}ResponseSeqRaw, size: usize) -> bool;
    fn {module_name}__srv__{type_name}_Response__Sequence__fini(msg: *mut {type_name}ResponseSeqRaw);
    fn rosidl_typesupport_c__get_service_type_support_handle__{module_name}__srv__{type_name}() -> *const rcl::rosidl_service_type_support_t;
    fn rosidl_typesupport_c__get_message_type_support_handle__{module_name}__srv__{type_name}_Request() -> *const rcl::rosidl_message_type_support_t;
    fn rosidl_typesupport_c__get_message_type_support_handle__{module_name}__srv__{type_name}_Response() -> *const rcl::rosidl_message_type_support_t;
}}
"
    );
    lines.push_back(cfun.into());
}

fn gen_impl_and_seq_msg(lines: &mut VecDeque<Cow<'_, str>>, module_name: &str, type_name: &str) {
    // generate impl and struct of sequence
    let impl_str = gen_impl(module_name, type_name, "", "", MsgOrSrv::Msg);
    let impl_trait_str = format!(
        "
impl TypeSupport for {type_name} {{
    fn type_support() -> *const rcl::rosidl_message_type_support_t {{
        unsafe {{
            rosidl_typesupport_c__get_message_type_support_handle__{module_name}__msg__{type_name}()
        }}
    }}
}}

impl PartialEq for {type_name} {{
    fn eq(&self, other: &Self) -> bool {{
        unsafe {{
            {module_name}__msg__{type_name}__are_equal(self, other)
        }}
    }}
}}

impl<const N: usize> PartialEq for {type_name}Seq<N> {{
    fn eq(&self, other: &Self) -> bool {{
        unsafe {{
            let msg1 = {type_name}SeqRaw{{data: self.data, size: self.size, capacity: self.capacity}};
            let msg2 = {type_name}SeqRaw{{data: other.data, size: other.size, capacity: other.capacity}};
            {module_name}__msg__{type_name}__Sequence__are_equal(&msg1, &msg2)
        }}
    }}
}}
"
    );

    lines.push_back(impl_str.into());
    lines.push_back(impl_trait_str.into());
}

fn gen_impl_and_seq_srv(lines: &mut VecDeque<Cow<'_, str>>, module_name: &str, type_name: &str) {
    // generate impl and struct of sequence
    let impl_str_req = gen_impl(module_name, type_name, "Request", "_Request", MsgOrSrv::Srv);
    let impl_str_resp = gen_impl(
        module_name,
        type_name,
        "Response",
        "_Response",
        MsgOrSrv::Srv,
    );

    lines.push_back(impl_str_req.into());
    lines.push_back(impl_str_resp.into());

    let struct_srv = format!(
        "
pub struct {type_name};

impl ServiceMsg for {type_name} {{
    type Request = {type_name}Request;
    type Response = {type_name}Response;
    fn type_support() -> *const rcl::rosidl_service_type_support_t {{
        unsafe {{
            rosidl_typesupport_c__get_service_type_support_handle__{module_name}__srv__{type_name}()
        }}
    }}
}}

impl TypeSupport for {type_name}Request {{
    fn type_support() -> *const rcl::rosidl_message_type_support_t {{
        unsafe {{
            rosidl_typesupport_c__get_message_type_support_handle__{module_name}__srv__{type_name}_Request()
        }}
    }}
}}

impl TypeSupport for {type_name}Response {{
    fn type_support() -> *const rcl::rosidl_message_type_support_t {{
        unsafe {{
            rosidl_typesupport_c__get_message_type_support_handle__{module_name}__srv__{type_name}_Response()
        }}
    }}
}}
"
    );

    lines.push_back(struct_srv.into());
}

#[derive(PartialEq, Eq)]
enum MsgOrSrv {
    Msg,
    Srv,
}

fn gen_impl(
    module_name: &str,
    type_name: &str,
    req_resp: &str,
    c_func_mid: &str,
    msg_or_srv: MsgOrSrv,
) -> String {
    let mid = if msg_or_srv == MsgOrSrv::Msg {
        "msg"
    } else {
        "srv"
    };

    let type_name_full = format!("{type_name}{req_resp}");

    format!(
        "
impl {type_name_full} {{
    pub fn new() -> Option<Self> {{
        let mut msg: Self = unsafe {{ std::mem::MaybeUninit::zeroed().assume_init() }};
        if unsafe {{ {module_name}__{mid}__{type_name}{c_func_mid}__init(&mut msg) }} {{
            Some(msg)
        }} else {{
            None
        }}
    }}
}}

impl Drop for {type_name_full} {{
    fn drop(&mut self) {{
        unsafe {{ {module_name}__{mid}__{type_name}{c_func_mid}__fini(self) }};
    }}
}}

#[repr(C)]
#[derive(Debug)]
struct {type_name_full}SeqRaw {{
    data: *mut {type_name_full},
    size: usize,
    capacity: usize,
}}

/// Sequence of {type_name_full}.
/// `N` is the maximum number of elements.
/// If `N` is `0`, the size is unlimited.
#[repr(C)]
#[derive(Debug)]
pub struct {type_name_full}Seq<const N: usize> {{
    data: *mut {type_name_full},
    size: usize,
    capacity: usize,
}}

impl<const N: usize> {type_name_full}Seq<N> {{
    /// Create a sequence of.
    /// `N` represents the maximum number of elements.
    /// If `N` is `0`, the sequence is unlimited.
    pub fn new(size: usize) -> Option<Self> {{
        if N != 0 && size >= N {{
            // the size exceeds in the maximum number
            return None;
        }}

        let mut msg: {type_name_full}SeqRaw = unsafe {{ std::mem::MaybeUninit::zeroed().assume_init() }};
        if unsafe {{ {module_name}__{mid}__{type_name}{c_func_mid}__Sequence__init(&mut msg, size) }} {{
            Some(Self {{data: msg.data, size: msg.size, capacity: msg.capacity }})
        }} else {{
            None
        }}
    }}

    pub fn null() -> Self {{
        let msg: {type_name_full}SeqRaw = unsafe {{ std::mem::MaybeUninit::zeroed().assume_init() }};
        Self {{data: msg.data, size: msg.size, capacity: msg.capacity }}
    }}

    pub fn as_slice(&self) -> &[{type_name_full}] {{
        if self.data.is_null() {{
            &[]
        }} else {{
            let s = unsafe {{ std::slice::from_raw_parts(self.data, self.size) }};
            s
        }}
    }}

    pub fn as_slice_mut(&mut self) -> &mut [{type_name_full}] {{
        if self.data.is_null() {{
            &mut []
        }} else {{
            let s = unsafe {{ std::slice::from_raw_parts_mut(self.data, self.size) }};
            s
        }}
    }}

    pub fn iter(&self) -> std::slice::Iter<'_, {type_name_full}> {{
        self.as_slice().iter()
    }}

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, {type_name_full}> {{
        self.as_slice_mut().iter_mut()
    }}

    pub fn len(&self) -> usize {{
        self.as_slice().len()
    }}

    pub fn is_empty(&self) -> bool {{
        self.len() == 0
    }}
}}

impl<const N: usize> Drop for {type_name_full}Seq<N> {{
    fn drop(&mut self) {{
        let mut msg = {type_name_full}SeqRaw{{data: self.data, size: self.size, capacity: self.capacity}};
        unsafe {{ {module_name}__{mid}__{type_name}{c_func_mid}__Sequence__fini(&mut msg) }};
    }}
}}

unsafe impl<const N: usize> Send for {type_name_full}Seq<N> {{}}
unsafe impl<const N: usize> Sync for {type_name_full}Seq<N> {{}}
"
    )
}
