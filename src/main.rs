use nom::{error::convert_error, Finish};

mod parser;

fn main() {
    let input = "
        # comment
# comment

std_msgs/Bool d
    int32 a #comment
    bool b #co

uint32 d 100
f32 e 20.99 # comment
i8 f = -5
i64[] arr1 = [10, 20, 30]
i64[3] arr2 = [10, 20, 30]
i64[<=3] arr3 = [10, 20, 30]
";

    match parser::parse_msg(input).finish() {
        Ok((_, result)) => println!("success: {:?}", result),
        Err(e) => println!("{}", convert_error(input, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::parser;
    use nom::Finish;

    #[test]
    fn test_msg() {
        let input = "
        # comment
# comment

std_msgs/Bool d
    int32 a #comment
    bool b #co

    ";
        let result = parser::parse_msg(input).finish().unwrap();
        println!("success: {:?}", result);
    }
}
