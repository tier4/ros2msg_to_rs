//! ros2msg_to_rs generates Rust files from ROS 2's .msg and .srv files.
//!
//! # How to use
//!
//! ## Step 1. Prepare .msg and .srv files
//!
//! ```text
//! $ mkdir src
//! $ mkdir src/my_module
//! $ mkdir src/my_module/msg
//! $ vi src/my_module/msg/example.msg
//! $ vi src/my_module/srv/example.srv
//! ```
//!
//! ## Step 2. Generate
//!
//! ```text
//! $ ros2msg_to_rs -i src -o target
//! $ ls target/module
//! mod.rs    msg.rs    srv.rs
//! ```
//!
//! `-i` is the input directory and `-o` is the output directory.
//! ros2msg_to_rs assumess the first first directories are modules.
//! If there is `src/my_module` and specify `-i src`,
//! ros2msg_to_rs assumes the `my_module` is a module.

use clap::Parser;
use convert_case::{Case, Casing};
use generator::Generator;
use nom::{error::convert_error, Finish};
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, VecDeque},
    error::Error,
    ffi::{OsStr, OsString},
    fs::{create_dir_all, File},
    io::prelude::*,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

mod generator;
mod parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Input directory containing .msg and .srv
    #[clap(short, long)]
    input: String,

    /// Path to the safe_drive.
    #[clap(short, long, default_value_t = String::from("safe_drive"))]
    safe_drive: String,

    /// Path to the output directory.
    #[clap(short, long, default_value_t = String::from("target"))]
    out: String,

    /// Disable to use common_interfaces. This option is used to generate common_interfaces used by safe_drive.
    /// So, do not set this option if you are not of the develeper of safe_drive.
    #[clap(long)]
    disable_common_interfaces: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let project_path = Path::new(&args.input).canonicalize()?;
    let project_name = project_path.file_name().unwrap();

    // destination directory
    let target = if args.out == "target" {
        Path::new("target").join(project_name)
    } else {
        Path::new(&args.out).to_path_buf()
    };

    let mod_dirs = generate_msgs(
        &target,
        &project_path,
        &args.safe_drive,
        args.disable_common_interfaces,
    )?;
    generate_mod_rs(&target, &mod_dirs)?;

    Ok(())
}

fn generate_mod_rs(
    target: &PathBuf,
    mod_dirs: &BTreeMap<PathBuf, BTreeSet<String>>,
) -> Result<(), Box<dyn Error>> {
    // create {target}/mod.rs
    let mod_rs_path = target.join("mod.rs");
    println!("generating: {}", mod_rs_path.display());
    let mut mod_rs = File::create(mod_rs_path)?;
    for (m, val) in mod_dirs.iter() {
        mod_rs.write_fmt(format_args!(
            "pub mod {};\n",
            m.file_name().unwrap().to_str().unwrap()
        ))?;

        // create {target}/{module}/mod.rs
        let mod_rs_in_path = m.join("mod.rs");
        println!("generating: {}", mod_rs_in_path.display());
        let mut mod_rs_in = File::create(mod_rs_in_path)?;

        for s in val {
            mod_rs_in.write_fmt(format_args!("pub mod {s};\n"))?;
            if s == "msg" {
                mod_rs_in.write_fmt(format_args!("pub use msg::*;\n"))?;
            }
        }
    }
    Ok(())
}

fn generate_msgs(
    target: &PathBuf,
    src: &PathBuf,
    safe_drive_path: &str,
    disable_common_interfaces: bool,
) -> Result<BTreeMap<PathBuf, BTreeSet<String>>, Box<dyn Error>> {
    let mut mod_name = OsString::new();
    let mut modules_msg = BTreeMap::new();
    let mut modules_srv = BTreeMap::new();
    let mut mod_dirs: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();

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
                if ext == "msg" || ext == "srv" {
                    if let Some(type_name) = p.file_name() {
                        let v: Vec<&str> = type_name.to_str().unwrap().split('.').collect();
                        let type_name = v.get(0).unwrap();

                        let mut f = File::open(p)?;
                        let mut contents = String::new();
                        f.read_to_string(&mut contents)?;

                        // generate Rust code
                        let mut g = Generator::new(
                            mod_name.to_str().unwrap().to_string(),
                            safe_drive_path.to_string(),
                            disable_common_interfaces,
                        );

                        let module_name = mod_name.to_str().unwrap();

                        let lines = if ext == "msg" {
                            generate_msg(&mut g, &contents, &path, module_name, type_name)?
                        } else {
                            generate_srv(&mut g, &contents, &path, module_name, type_name)?
                        };

                        // "{target}/{mod_name}"
                        let mod_dir = target.join(mod_name.to_str().unwrap());

                        if let Some(mods) = mod_dirs.get_mut(&mod_dir) {
                            mods.insert(ext.to_str().unwrap().to_string());
                        } else {
                            let mut mods = BTreeSet::new();
                            mods.insert(ext.to_str().unwrap().to_string());
                            mod_dirs.insert(mod_dir.clone(), mods);
                        }

                        // module's directory
                        // {target}/{mod_name}/(msg|srv)
                        let target_dir = if ext == "msg" {
                            mod_dir.join("msg")
                        } else {
                            mod_dir.join("srv")
                        };

                        // create directory
                        create_dir_all(&target_dir)?;

                        // generate {target}/{mod_name}/(msg|srv)/{snake_type_name}.rs
                        let sname = type_name.to_case(Case::Snake);
                        let snake_type_name = mangle(&sname);

                        let mod_file = format!("{snake_type_name}.rs");
                        let target_file = target_dir.join(mod_file);

                        add_modules(
                            if ext == "msg" {
                                &mut modules_msg
                            } else {
                                &mut modules_srv
                            },
                            mod_dir.as_os_str(),
                            snake_type_name.to_string(),
                        );

                        let mut w = File::create(&target_file)?;

                        println!("generating: {}", target_file.display());
                        for line in lines {
                            w.write_fmt(format_args!("{}\n", line))?;
                        }
                    }
                }
            }
        }
    }

    for (k, v) in modules_msg {
        generate_msg_srv_rs(&v, &Path::new(&k).join("msg.rs"))?;
    }

    for (k, v) in modules_srv {
        generate_msg_srv_rs(&v, &Path::new(&k).join("srv.rs"))?;
    }

    Ok(mod_dirs)
}

fn generate_msg<'a>(
    generator: &mut Generator,
    contents: &str,
    path: &walkdir::DirEntry,
    module_name: &'a str,
    type_name: &'a str,
) -> Result<VecDeque<Cow<'a, str>>, Box<dyn Error>> {
    match parser::parse_msg(&contents).finish() {
        Ok((_, exprs)) => Ok(generator.gen_msg(module_name, type_name, &exprs)),
        Err(e) => {
            eprintln!("{}", convert_error(contents, e));
            let msg = format!("failed to parse: {}", path.path().display());
            return Err(msg.into());
        }
    }
}

fn generate_srv<'a>(
    generator: &mut Generator,
    contents: &str,
    path: &walkdir::DirEntry,
    module_name: &'a str,
    type_name: &'a str,
) -> Result<VecDeque<Cow<'a, str>>, Box<dyn Error>> {
    match parser::parse_srv(&contents).finish() {
        Ok((_, (exprs_req, exprs_resp))) => {
            Ok(generator.gen_srv(module_name, type_name, &exprs_req, &exprs_resp))
        }
        Err(e) => {
            eprintln!("{}", convert_error(contents, e));
            let msg = format!("failed to parse: {}", path.path().display());
            return Err(msg.into());
        }
    }
}

fn add_modules(map: &mut BTreeMap<OsString, Vec<String>>, key: &OsStr, value: String) {
    if let Some(v) = map.get_mut(key) {
        v.push(value);
    } else {
        let v = vec![value];
        map.insert(key.to_os_string(), v);
    }
}

fn generate_msg_srv_rs(modules: &[String], target_file: &PathBuf) -> Result<(), Box<dyn Error>> {
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

pub fn mangle(var_name: &str) -> Cow<'_, str> {
    match var_name {
        "type" | "pub" | "fn" | "match" | "if" | "while" | "break" | "continue" | "unsafe"
        | "async" | "move" | "trait" | "impl" | "for" | "i8" | "u8" | "i16" | "u16" | "i32"
        | "u32" | "i64" | "u64" | "bool" | "char" => format!("{var_name}_").into(),
        _ => var_name.into(),
    }
}

#[cfg(test)]
mod tests {
    use crate::generator::Generator;

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
        let mut g = Generator::new("my_library".to_string(), "crate".to_string(), false);
        let (_, exprs) = parser::parse_msg(input).finish().unwrap();
        g.gen_msg("TestModule", "TestMsg", &exprs);
    }
}
