#![allow(dead_code)]
mod sections;
use crate::sections::*;
use serde_json;
use std::{
    env,
    fs::{read_to_string, File},
    io::Write,
};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 4 || (args[1] != "decode" && args[1] != "encode") {
        println!("Usage: <Decode/Encode> <Input File> <Output File>");
        return;
    }
    let (input_filename, output_filename) = (&args[2], &args[3]);

    if args[1] == "decode" {
        let input_file = File::open(input_filename).unwrap();
        let kmp = KMP::new(input_file).unwrap();
        let mut kmp_json_file = File::create(output_filename).unwrap();
        let kmp_json = serde_json::to_string_pretty(&kmp).unwrap();
        kmp_json_file.write(kmp_json.as_bytes()).unwrap();
    } else if args[1] == "encode" {
        let kmp_json = read_to_string(input_filename).unwrap();
        let kmp: KMP = serde_json::from_str(&kmp_json).unwrap();
        let mut output_file = File::create(output_filename).unwrap();
        kmp.write(&mut output_file).unwrap();
    }
}
