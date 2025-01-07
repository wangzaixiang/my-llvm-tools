use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

fn main() -> io::Result<()> {
    // args[1] is the input file like abc.ll
    let input_file = std::env::args().nth(1).expect("no filename given");
    if !input_file.ends_with(".ll") {
        panic!("input file must end with .ll");
    }

    let path = std::path::Path::new(&input_file);
    let basename = path.file_stem().expect("no basename found").to_str().expect("basename is not a valid UTF-8 string");

    let file = File::open(input_file.as_str())?;
    let reader = BufReader::new(file);

    let mut file_count = 0;
    let mut output_file = File::create(format!("./output/{basename}_{file_count}.ll"))?;

    for line in reader.lines() {
        let line = line?;
        if line.contains(" Dump After ") {
            file_count += 1;
            output_file = File::create(format!("./output/{basename}_{file_count}.ll"))?;
        }
        writeln!(output_file, "{}", line)?;
    }

    Ok(())
}