use convert_case::{Case, Casing};
use generator::gen_msg;
use nom::{error::convert_error, Finish};
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    ffi::{OsStr, OsString},
    fs::{create_dir_all, File},
    io::prelude::*,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

mod generator;
mod parser;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        eprintln!("usage: {} src [dst]", args[0]);
        eprintln!("if [dst] is omitted, files are created under \"target\"");
        return Err("at least one argument (src) is required".into());
    }

    let arg_path = Path::new(&args[1]).canonicalize()?;
    let project_name = arg_path.file_name().unwrap();

    // destination directory
    let target = if let Some(dst) = args.get(2) {
        Path::new(dst).to_path_buf()
    } else {
        Path::new("target").join(project_name)
    };

    generate_msgs(&target, &arg_path)?;

    Ok(())
}

fn generate_msgs(target: &PathBuf, src: &PathBuf) -> Result<(), Box<dyn Error>> {
    let mut mod_name = OsString::new();
    let mut modules = BTreeMap::new();

    // traverse directory
    for entry in WalkDir::new(src) {
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

                        // parse .msg file
                        match parser::parse_msg(&contents).finish() {
                            Ok((_, result)) => {
                                // generate Rust code
                                let lines = gen_msg(mod_name.to_str().unwrap(), type_name, &result);

                                // crate's src directory
                                // "target/{mod_name}/src"
                                let crate_dir = target.join(mod_name.to_str().unwrap()).join("src");

                                // module's directory
                                // "target/{mod_name}/src/msg"
                                let target_dir = crate_dir.join("msg");

                                // create directory
                                create_dir_all(&target_dir)?;

                                // generate target/{mod_name}/src/msg/{snake_type_name}.rs
                                let snake_type_name = type_name.to_case(Case::Snake);
                                let mod_file = format!("{snake_type_name}.rs");
                                let target_file = target_dir.join(mod_file);

                                add_modules(&mut modules, crate_dir.as_os_str(), snake_type_name);

                                let mut w = File::create(&target_file)?;

                                println!("generating: {}", target_file.display());
                                for line in lines {
                                    w.write_fmt(format_args!("{}\n", line))?;
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

    for (k, v) in modules {
        generate_msg_rs(&v, Path::new(&k))?;
    }

    Ok(())
}

fn add_modules(map: &mut BTreeMap<OsString, Vec<String>>, key: &OsStr, value: String) {
    if let Some(v) = map.get_mut(key) {
        v.push(value);
    } else {
        let v = vec![value];
        map.insert(key.to_os_string(), v);
    }
}

fn generate_msg_rs(modules: &[String], target_dir: &Path) -> Result<(), Box<dyn Error>> {
    let target_file = target_dir.join("msg.rs");
    let mut w = File::create(&target_file)?;

    println!("generating: {}", target_file.display());

    for module in modules.iter() {
        w.write_fmt(format_args!("mod {};\n", module))?;
    }

    w.write("\n".as_bytes())?;

    for module in modules.iter() {
        w.write_fmt(format_args!("pub use {}::*;\n", module))?;
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
