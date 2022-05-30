use nom::{
    branch::alt,
    bytes::complete::tag,
    character::{
        self,
        complete::{
            alpha1, anychar, line_ending, not_line_ending, one_of, satisfy, space0, space1,
        },
        is_alphanumeric, is_digit,
    },
    combinator::peek,
    error::VerboseError,
    multi::{many0, separated_list1},
    number,
    sequence::{delimited, preceded},
    IResult,
};
use std::fmt::Display;

type PResult<'a, OUT> = IResult<&'a str, OUT, VerboseError<&'a str>>;

#[derive(Debug)]
pub enum Expr {
    Variable {
        type_name: TypeName,
        var_name: String,
        value: Option<ValueType>,
        comment: Option<String>,
    },
    Empty, // comment or empty line
    Comment,
    Eof,
}

#[derive(Debug)]
pub enum ValueType {
    Const(Value),
    Default(Value),
}

#[derive(Debug)]
pub enum Value {
    Bool(bool),
    String(String),
    Float(f64),
    Uint(u64),
    Int(i64),
    Array(Vec<Value>),
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{n}"),
            Value::Uint(n) => write!(f, "{n}"),
            Value::Float(n) => write!(f, "{n}"),
            Value::String(n) => write!(f, "b\"{n}\0\""),
            Value::Bool(n) => write!(f, "{n}"),
            Value::Array(n) => write!(f, "{:?}", n),
        }
    }
}

#[derive(Debug)]
pub enum TypeName {
    Type {
        type_name: String,
        array_info: ArrayInfo,
    },
    ScopedType {
        scope: String,
        type_name: String,
        array_info: ArrayInfo,
    },
    LimitedString {
        size: usize,
        array_info: ArrayInfo,
    },
    String(ArrayInfo),
}

#[derive(Debug)]
pub enum ArrayInfo {
    NotArray,
    Dynamic,
    Static(usize),
    Limited(usize),
}

/// Parse .msg file.
///
/// # Grammar
///
/// ```text
/// $Msg = $Expr $Expr | $Expr
/// ```
pub fn parse_msg(mut input: &str) -> PResult<Vec<Expr>> {
    let mut result = Vec::new();
    loop {
        if input.is_empty() {
            break;
        }

        let (next, expr) = parse_expr(input)?;
        input = next;

        if let Expr::Variable { .. } = &expr {
            result.push(expr);
        }
    }

    Ok((input, result))
}

/// ```text
/// $Expr = $Empty | $Comment | $VarDef
///
/// $Empty | $Comment | $VarDef
///
/// $VarDef = $Variable $Comment $End | $Variable $End
/// $Variable = $TypeName $ID | $TypeName $ID = $Value | $TypeName $ID $Value
/// $TypeName =
///     string<=$PlusNum |
///     string<=$PlusNum $ArrayInfo |
///     $ID/$ID $ArrayInfo |
///     $ID/$ID |
///     $ID $ArrayInfo |
///     $ID
/// $ArrayInfo = [] | [$PlusNum] | [<=$PlusNum]
/// $PlusNum = Regex([0..9]+)
///
/// $Comment = Regex(#.*) $End
///
/// $ID = Regex((_|[a..zA..Z]+)([a..zA..Z0..9]|_)*)
///
/// $Value = $Bool | $Num | $Array | $String
/// $Bool = true | false
/// $Num = Regex(-?[0..9]+(.[0..9]+)?)
/// $String = 'characters' | "characters"
///
/// $Array = [ $Elements ]
/// $Elements = $Value | $Value , $Elements
/// ```
fn parse_expr(input: &str) -> PResult<Expr> {
    let (input, _) = space0(input)?;
    alt((parse_empty, parse_comment, parse_variable))(input)
}

/// ```text
/// $VarDef = $Variable $Comment $End | $Variable $End
/// $Variable = $TypeName $ID | $TypeName $ID = $Value | $TypeName $ID $Value
/// ```
fn parse_variable(input: &str) -> PResult<Expr> {
    let (input, type_name) = parse_typename(input)?;

    // skip whitespaces
    let (input, _) = space1(input)?;

    // parse variable name
    // $ID
    let (input, var_name) = parse_identifier(input)?;

    // skip whitespaces
    let (input, _) = space0(input)?;

    // parse default or constant value
    let mut value = None;
    let input = if let Ok((_, c)) = peek_next_of(input) {
        match c {
            '=' => {
                // constant value
                // = $Value
                let (input, _) = tag("=")(input)?;
                let (input, _) = space0(input)?;
                let (input, val) = parse_value(input)?;
                value = Some(ValueType::Const(val));
                input
            }
            '"' => todo!(),
            _ => {
                // default value
                // $Value
                let (input, val) = parse_value(input)?;
                value = Some(ValueType::Default(val));
                input
            }
        }
    } else {
        input
    };

    // skip whitespaces
    let (input, _) = space0(input)?;

    let (input, comment) = if peek_tag("#", input).is_ok() {
        // skip coment
        let (input, _) = tag("#")(input)?;
        let (input, c) = not_line_ending(input)?;
        (input, Some(c.to_string()))
    } else {
        (input, None)
    };

    let input = if !input.is_empty() {
        // skip line ending
        let (input, _) = line_ending(input)?;
        input
    } else {
        input
    };

    Ok((
        input,
        Expr::Variable {
            type_name,
            var_name,
            value,
            comment,
        },
    ))
}

/// ```text
/// $TypeName =
///     string<=$PlusNum |
///     string<=$PlusNum $ArrayInfo |
///     $ID/$ID $ArrayInfo |
///     $ID/$ID |
///     $ID $ArrayInfo |
///     $ID
/// ```
fn parse_typename(input: &str) -> PResult<TypeName> {
    // parse type name
    let (input, scope) = parse_identifier(input)?;

    if scope == "string" {
        return parse_string_type(input);
    }

    if peek_tag("/", input).is_ok() {
        // $ID/$ID
        let (input, _) = tag("/")(input)?;
        let (input, type_name) = parse_identifier(input)?;

        // $ArrayInfo
        //let (input, _) = space0(input)?;
        let (input, array_info) = parse_array_info(input)?;
        Ok((
            input,
            TypeName::ScopedType {
                scope,
                type_name,
                array_info,
            },
        ))
    } else {
        // $ArrayInfo
        let (input, array_info) = parse_array_info(input)?;
        Ok((
            input,
            TypeName::Type {
                type_name: scope,
                array_info,
            },
        ))
    }
}

fn parse_string_type(input: &str) -> PResult<TypeName> {
    if peek_tag("<=", input).is_ok() {
        let (input, _) = tag("<=")(input)?;
        let (input, size) = character::complete::u64(input)?;
        let (input, array_info) = parse_array_info(input)?;
        Ok((
            input,
            TypeName::LimitedString {
                size: size as usize,
                array_info,
            },
        ))
    } else {
        let (input, array_info) = parse_array_info(input)?;
        Ok((input, TypeName::String(array_info)))
    }
}

/// ```text
/// $ID = Regex((_|[a..zA..Z]+)([a..zA..Z0..9]|_)*)
/// ```
fn parse_identifier(input: &str) -> PResult<String> {
    // (_|[a..zA..Z]+)
    let (input, head) = alt((tag("_"), alpha1))(input)?;

    // [a..zA..Z0..9_]*
    let (input, tail) = many0(satisfy(|c| is_alphanumeric(c as u8) || c == '_'))(input)?;

    let tail: String = tail.iter().collect();
    Ok((input, head.to_string() + &tail))
}

/// ```text
/// $Comment = Regex(#.*) $End
/// ```
fn parse_comment(input: &str) -> PResult<Expr> {
    let (input, _) = tag("#")(input)?;
    let (input, _) = not_line_ending(input)?;

    let input = if !input.is_empty() {
        // skip line ending
        let (input, _) = line_ending(input)?;
        input
    } else {
        input
    };

    Ok((input, Expr::Comment))
}

/// empty line or EOF
fn parse_empty(input: &str) -> PResult<Expr> {
    if input.is_empty() {
        Ok((input, Expr::Eof))
    } else {
        let (input, _) = line_ending(input)?;
        Ok((input, Expr::Empty))
    }
}

fn peek_tag<'a>(c: &'static str, input: &'a str) -> PResult<'a, &'a str> {
    peek(tag(c))(input)
}

fn peek_next_of(input: &str) -> PResult<char> {
    peek(alt((one_of("=-\"'[tf"), digit)))(input)
}

fn digit(input: &str) -> PResult<char> {
    satisfy(|c| is_digit(c as u8))(input)
}

/// ```text
/// $Value = $Bool | $Num | $Array | $String
/// $Bool = true | false
/// $Num = Regex(-?[0..9]+(.[0..9]+)?)
/// $String = 'characters' | "characters"
/// ```
fn parse_value(input: &str) -> PResult<Value> {
    alt((parse_num, parse_bool, parse_array, parse_string))(input)
}

/// ```text
/// $Num = Regex(-?[0..9]+(.[0..9]+)?)
/// ```
fn parse_num(input: &str) -> PResult<Value> {
    // parse minus
    let (input, minus) = if peek_tag("-", input).is_ok() {
        let (input, _) = tag("-")(input)?;
        (input, -1)
    } else {
        (input, 1)
    };

    let (input, n) = character::complete::u64(input)?;
    if peek_tag(".", input).is_ok() {
        let (input, d) = number::complete::double(input)?;
        let val = (d + n as f64) * minus as f64;
        Ok((input, Value::Float(val)))
    } else if minus == -1 {
        Ok((input, Value::Int(n as i64 * minus)))
    } else {
        Ok((input, Value::Uint(n)))
    }
}

/// ```text
/// $Bool = true | false
/// ```
fn parse_bool(input: &str) -> PResult<Value> {
    let (input, val) = alt((tag("true"), tag("false")))(input)?;
    if val == "true" {
        Ok((input, Value::Bool(true)))
    } else {
        Ok((input, Value::Bool(false)))
    }
}

/// ```text
/// $Array = [ $Elements ]
/// $Elements = $Value | $Elements , $Value
/// ```
fn parse_array(input: &str) -> PResult<Value> {
    let p = delimited(space0, parse_value, space0);
    let (input, val) = delimited(tag("["), separated_list1(tag(","), p), tag("]"))(input)?;
    Ok((input, Value::Array(val)))
}

/// ```text
/// $ArrayInfo = [] | [$PlusNum] | [<=$PlusNum]
/// $PlusNum = Regex([0..9]+)
/// ```
fn parse_array_info(input: &str) -> PResult<ArrayInfo> {
    fn is_array(input: &str) -> PResult<()> {
        let (input, _) = peek(preceded(space0, tag("[")))(input)?;
        Ok((input, ()))
    }

    if is_array(input).is_ok() {
        let (input, _) = space0(input)?;
        let (input, _) = tag("[")(input)?;
        let (input, _) = space0(input)?;
        let (input, array_info) = if peek_tag("]", input).is_ok() {
            // []
            (input, ArrayInfo::Dynamic)
        } else if peek_tag("<", input).is_ok() {
            // [<=$PlusNum]
            let (input, _) = tag("<=")(input)?;
            let (input, size) = character::complete::u64(input)?;
            (input, ArrayInfo::Limited(size as usize))
        } else {
            // [$PlusNum]
            let (input, size) = character::complete::u64(input)?;
            (input, ArrayInfo::Static(size as usize))
        };

        let (input, _) = space0(input)?;
        let (input, _) = tag("]")(input)?;

        Ok((input, array_info))
    } else {
        Ok((input, ArrayInfo::NotArray))
    }
}

/// # Escaped Characters
///
/// - \\
/// - \r
/// - \n
/// - \t
/// - \' or \"
fn parse_string(input: &str) -> PResult<Value> {
    let (mut input, quote) = one_of("\"'")(input)?;

    let mut val = String::new();

    loop {
        let (next, c) = anychar(input)?;
        input = next;

        match c {
            c if c == quote => return Ok((input, Value::String(val))),
            '\\' => {
                let (next, c) = alt((one_of("rnt\\"), character::complete::char(quote)))(input)?;
                input = next;

                match c {
                    'r' => {
                        val.push_str("\\r");
                    }
                    'n' => {
                        val.push_str("\\n");
                    }
                    't' => {
                        val.push_str("\\t");
                    }
                    '\\' => {
                        val.push_str("\\\\");
                    }
                    c if c == quote => {
                        if c == '"' {
                            val.push('\\');
                        }
                        val.push(quote);
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                val.push(c);
            }
        }
    }
}
