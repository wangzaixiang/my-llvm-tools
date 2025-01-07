use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::Path;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(about, version, author)]
struct Args {
    /// The input LLVM IR file.
    input: String,

    /// whether to include instructions inside basic blocks.
    #[arg(long, default_value = "false")]
    abbr: bool,

    /// The function to generate the CFG for. if not specified, all functions are considered.
    #[arg(short, long)]
    function: Option<String>,

    /// The output file(markdown) to write the CFG to. If not specified, the CFG is written to stdout.
    #[arg(short, long)]
    output: Option<String>,
}

type BlockName = String;

#[derive(Clone, Debug)]
struct BasicBlock {
    name: BlockName,  // entry has name ""
    instructions: Vec<String>,
    predecessors: Vec<BlockName>,
    successors: Vec<BlockName>,
}

#[derive(Debug)]
#[allow(dead_code)]
struct Function {
    name: String,
    define: String, // the define line
    blocks: Vec<BasicBlock>,
}

#[allow(dead_code)]
impl Function {
    fn to_string(&self) -> String {
        use std::fmt::Write;
        let mut buffer = String::new();

        _ = writeln!(buffer, "Function: {}", self.name);
        for block in &self.blocks {
            _ = writeln!(buffer, "\tBlock: {}\t; preds = {}", block.name, block.predecessors.join(", "));
            for instr in &block.instructions {
                _ = writeln!(buffer, "\t\t  {}", instr);
            }
            _ = writeln!(buffer, "\t; successors = {}", block.successors.join(", "));
        }

        buffer
    }

}


fn main() -> io::Result<()> {

    let args = Args::parse();

    if !Path::new(&args.input).exists() {
        eprintln!("Input file does not exist: {}", args.input);
        std::process::exit(1);
    }

    let mut reader = io::BufReader::new( File::open(&args.input)? );
    let result = parse_ll_file(&mut reader)?;

    let output: &mut dyn Write = if let Some(output) = &args.output {
        &mut File::create(output)?
    }
    else {
        &mut io::stdout()
    };

    if let Some(func_name) = &args.function {
        result.iter().filter(|f| f.name == *func_name).for_each(|f| {
            dump_cfg(output, f, args.abbr);
        });
    }
    else {
        result.iter().for_each(|f| {
            dump_cfg(output, f, args.abbr);
        });
    }

    Ok(())
}

fn dump_cfg(output: &mut dyn Write, function: &Function, abbr: bool)  {
    _ = writeln!(output, "```mermaid");
    _ = writeln!(output, "flowchart TD");
    _ = writeln!(output, "%% function {}", function.name);
    function.blocks.iter().for_each(|block| {
        let block_name = if block.name == "" { "%1" } else { &format!("%{}", &block.name) };
        block.predecessors.iter().for_each(|src_name|
            _ = writeln!(output, "\t{} -->|{}| {}", src_name, block_name, block_name)
        );
        if abbr == false {
            let block_label = block.instructions.join("\n");
            _ = writeln!(output, "{}[\"{}\"]", block_name, block_label);
        }
    });
    _ = writeln!(output, "```").unwrap();
}

fn parse_ll_file<R: Read>(reader: &mut io::BufReader<R>) -> io::Result<Vec<Function>>{

    let define_re = Regex::new(r"^define\s+.*@([a-zA-Z0-9_]+)\s*\(").unwrap();

    let mut functions: Vec<Function> = vec![];

    let mut lines = reader.lines();
    while let Some(line) = lines.next() {
        let line = line?;
        if let Some(caps) = define_re.captures(&line) {
            if let Some(func_name) = caps.get(1).map(|m| m.as_str().to_string()) {
                let blocks = parse_blocks(&mut lines);
                let current_function = Function {
                    name: func_name.clone(),
                    define: line.clone(),
                    blocks,
                };
                functions.push(current_function);
            }
        }
        else {
            // skip
        }
    }

    Ok(functions)
}

fn parse_blocks<R: Read>(lines: &mut io::Lines<&mut BufReader<R>>) -> Vec<BasicBlock> {
    let block_name_re = Regex::new(r"^([0-9]+):\s*; preds = (.*)$").unwrap();
    let jump_re = Regex::new(r"^\s*br\s+(.*)").unwrap();

    let mut blocks: Vec<BasicBlock> = vec![];
    let mut current_block = BasicBlock {
        name: "".to_string(),
        instructions: vec![],
        predecessors: vec![],
        successors: vec![],
    };

    while let Some(line) = lines.next() {
        let line = line.unwrap();
        if let Some(caps) = block_name_re.captures(&line) {
            if let Some(block_name) = caps.get(1).map(|m| m.as_str().to_string()) {
                blocks.push(current_block.clone());
                current_block = BasicBlock {
                    name: block_name.clone(),
                    instructions: vec![],
                    predecessors: vec![],
                    successors: vec![],
                };

                let preds = caps.get(2).map(|m| m.as_str().to_string());
                if let Some(preds) = preds {
                    let preds = preds.split(", ").map(|s| s.to_string()).collect();
                    current_block.predecessors = preds;
                }
            }
        } else if line == "}" {
                blocks.push(current_block.clone());
                break;
        }
        else {
            current_block.instructions.push(line.clone());
            if let Some(caps) = jump_re.captures(&line) {
                if let Some(jump_content) = caps.get(1).map(|m| m.as_str().to_string()) {
                    jump_content.split(',').filter(|s| s.contains("label ")).for_each(|s| {
                        let jump_to = s.split_whitespace().last().unwrap().to_string();
                        current_block.successors.push(jump_to);
                    });
                }
            }
        }
    }

    blocks.push(current_block);

    blocks
}
