extern crate getopts;
extern crate peep;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use getopts::Options;
use peep::app::App;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

fn main() -> io::Result<()> {
    // preare streams
    let stdout_stream = io::stdout();
    let mut stdoutlock_stream = stdout_stream.lock();
    let stdin_stream = io::stdin();
    let mut stdinlock_stream = stdin_stream.lock();

    // parse command arguments
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optopt("n", "numof-lines", "number of lines", "NUMBER");
    opts.optflag("N", "print-number", "print line number");
    opts.optflag("v", "print-nonprinting", "print non-printing");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => { panic!(f.to_string()) }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return Ok(());
    }

    let mut app = App::new();

    if let Ok(Some(nlines)) = matches.opt_get::<u32>("n") {
        app.set_numof_lines(nlines);
    }
    if matches.opt_present("v") {
        app.set_show_nonprinting(true);
    }
    if matches.opt_present("N") {
        app.set_show_line_number(true);
    }

    let file_name = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        "-".to_owned()
    };
    let file;
    let mut freader;
    let mut bufreader: &mut BufRead = if file_name == "-" {
        // &mut stdinlock_stream
        print_usage(&program, opts);
        unimplemented!()
    } else {
        file = File::open(&file_name).expect(&format!(
                "{}: {}: No such file or directory",
                program, file_name));
        freader = BufReader::new(file);
        &mut freader
    };

    app.run(&mut stdinlock_stream, &mut bufreader, &mut stdoutlock_stream);

    Ok(())
}
