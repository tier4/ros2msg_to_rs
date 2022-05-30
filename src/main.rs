use generator::gen_msg;
use nom::{error::convert_error, Finish};

mod generator;
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

string s1 = \"abc\\\\ def \\\" ghi \"
string s2 = \"\\r\\n\\t\"
string<=10 s3
string<=10 [5] s4

";

    let input2 = "
bool a
string b
int8 c
uint8 d
int16 e
uint16 f
int32 g
uint32 h
int64 i # aaa
uint64 k
float32 l
float64 m

bool o = true
float64 p = 10.2

string s1 = \"abc\\\\ def \\\" ghi \" # bbbb
string s2 = \"\\r\\n\\t\"
";

    match parser::parse_msg(input2).finish() {
        Ok((_, result)) => {
            println!("success: {:#?}", result);

            let lines = gen_msg("TestMsg", &result);
            for line in lines {
                println!("{line}");
            }
        }
        Err(e) => println!("{}", convert_error(input2, e)),
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
