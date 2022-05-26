use nom::{
    branch::alt,
    bytes::complete::tag,
    character::{
        self,
        complete::{alpha1, line_ending, not_line_ending, one_of, satisfy, space0, space1},
        is_alphanumeric, is_digit,
    },
    combinator::peek,
    error::VerboseError,
    multi::{many0, separated_list1},
    number,
    sequence::delimited,
    IResult,
};

type PResult<'a, OUT> = IResult<&'a str, OUT, VerboseError<&'a str>>;

#[derive(Debug)]
pub enum Expr {
    Variable {
        type_name: TypeName,
        var_name: String,
        value: Option<ValueType>,
    },
    Empty, // comment or empty line
    Comment,
    EOF,
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
    LimitedString(usize),
}

#[derive(Debug)]
pub enum ArrayInfo {
    NotArray,
    DynamicArray,
    StaticArray(usize),
    LimitedArray(usize),
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
/// $TypeName = $ID/$ID $ArrayInfo | $ID/$ID | $ID $ArrayInfo | $ID
/// $ArrayInfo = [] | [$PlusNum] | [<=$PlusNum]
/// $PlusNum = Regex([0..9]+)
///
/// $Comment = Regex(#.*) $End
///
/// $ID = Regex((_|[a..zA..Z]+)[a..zA..Z0..9]*)
///
/// $Value = $Bool | $Num | $Array
/// $Bool = true | false
/// $Num = Regex(-?[0..9]+(.[0..9]+)?)
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
/// $TypeName = $ID/$ID $ArrayInfo | $ID/$ID | $ID $ArrayInfo | $ID
/// ```
fn parse_variable(input: &str) -> PResult<Expr> {
    // parse type name
    // $TypeName = $ID/$ID $ArrayInfo | $ID/$ID | $ID $ArrayInfo | $ID
    let (input, scope) = parse_identifier(input)?;

    let (input, type_name) = if peek_tag("/", input).is_ok() {
        // $ID/$ID
        let (input, _) = tag("/")(input)?;
        let (input, type_name) = parse_identifier(input)?;

        // $ArrayInfo
        let (input, array_info) = parse_array_info(input)?;
        (
            input,
            TypeName::ScopedType {
                scope,
                type_name,
                array_info,
            },
        )
    } else {
        // $ArrayInfo
        let (input, array_info) = parse_array_info(input)?;
        (
            input,
            TypeName::Type {
                type_name: scope,
                array_info,
            },
        )
    };

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

    let input = if let Ok(_) = peek_tag("#", input) {
        // skip coment
        let (input, _) = tag("#")(input)?;
        let (input, _) = not_line_ending(input)?;
        input
    } else {
        input
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
        },
    ))
}

/// ```text
/// $ID = Regex((_|[a..zA..Z]+)[a..zA..Z0..9]*)
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
        Ok((input, Expr::EOF))
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

fn parse_value(input: &str) -> PResult<Value> {
    alt((parse_num, parse_bool, parse_array))(input)
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
    } else {
        if minus == -1 {
            Ok((input, Value::Int(n as i64 * minus)))
        } else {
            Ok((input, Value::Uint(n)))
        }
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
    if peek_tag("[", input).is_ok() {
        let (input, _) = tag("[")(input)?;
        let (input, _) = space0(input)?;
        let (input, array_info) = if peek_tag("]", input).is_ok() {
            // []
            (input, ArrayInfo::DynamicArray)
        } else if peek_tag("<", input).is_ok() {
            // [<=$PlusNum]
            let (input, _) = tag("<=")(input)?;
            let (input, size) = character::complete::u64(input)?;
            (input, ArrayInfo::LimitedArray(size as usize))
        } else {
            // [$PlusNum]
            let (input, size) = character::complete::u64(input)?;
            (input, ArrayInfo::StaticArray(size as usize))
        };

        let (input, _) = space0(input)?;
        let (input, _) = tag("]")(input)?;

        Ok((input, array_info))
    } else {
        Ok((input, ArrayInfo::NotArray))
    }
}
