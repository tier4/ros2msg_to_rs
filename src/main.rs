use generator::gen_msg;
use nom::{error::convert_error, Finish};
use std::{env, error::Error, ffi::OsString, fs::File, io::prelude::*};
use walkdir::WalkDir;

mod generator;
mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        eprintln!("usage: {} dir", args[0]);
        return Err("an argument is required".into());
    }

    let mut mod_name = OsString::new();

    // traverse directory
    for entry in WalkDir::new(&args[1]) {
        let path = entry?;

        // assume children are modules
        if path.depth() == 1 {
            mod_name = path.file_name().to_os_string();
        }

        if path.file_type().is_file() {
            let p = path.path();

            // transpile .msg file
            if let Some(ext) = p.extension() {
                if ext == "msg" {
                    if let Some(type_name) = p.file_name() {
                        let v: Vec<&str> = type_name.to_str().unwrap().split('.').collect();
                        let type_name = v.get(0).unwrap();

                        let mut f = File::open(p)?;
                        let mut contents = String::new();
                        f.read_to_string(&mut contents)?;

                        match parser::parse_msg(&contents).finish() {
                            Ok((_, result)) => {
                                println!("{}", path.path().display());

                                let lines = gen_msg(mod_name.to_str().unwrap(), type_name, &result);
                                for line in lines {
                                    println!("{line}");
                                }
                            }
                            Err(e) => {
                                eprintln!("{}", convert_error(contents.as_str(), e));
                                let msg = format!("failed to parse: {}", path.path().display());
                                return Err(msg.into());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::generator::gen_msg;

    use super::parser;
    use nom::Finish;

    #[test]
    fn test_msg() {
        let input1 = "
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
string<=10 s3

i32[] array1
i32[10] array2
string[] array3
string<=10[3] array4
string<=10[] array5

std_msgs/Bool std1
std_msgs/Bool std2
std_msgs/Header std3
";

        generate(input1);
        generate(input2);
    }

    fn generate(input: &str) {
        let (_, exprs) = parser::parse_msg(input).finish().unwrap();
        gen_msg("TestModule", "TestMsg", &exprs);
    }
}
