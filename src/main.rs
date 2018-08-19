extern crate getopts;
extern crate peep;
extern crate termion;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use getopts::Options;
use peep::app::App;
use peep::tty;

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} FILE [options]", program);
    print!("{}", opts.usage(&brief));
}

fn read_buffer(filename: &str) -> io::Result<Vec<String>> {
    let mut buffer: Vec<String> = vec![];

    if filename == "-" {
        let inp = io::stdin();
        if termion::is_tty(&inp) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "no input"));
        }
        let inp = inp.lock();
        for v in inp.lines().map(|v| v.unwrap()) {
            buffer.push(v);
        }
    } else {
        if let Ok(file) = File::open(&filename) {
            let mut bufreader = BufReader::new(file);
            for v in bufreader.lines().map(|v| v.unwrap()) {
                buffer.push(v);
            }
        } else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
        }
    }

    return Ok(buffer)
}

fn main() -> io::Result<()> {
    // parse command arguments
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();
    let mut opts = Options::new();
    opts.optopt("n", "numof-lines", "number of lines", "NUMBER");
    opts.optflag("N", "print-number", "print line number");
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
    if matches.opt_present("N") {
        app.set_show_line_number(true);
    }

    let file_name = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        "-".to_owned()
    };
    let buffer = read_buffer(&file_name).unwrap();

    // preare streams
    let ttyout = io::stdout();
    let mut ttyout = ttyout.lock();

    tty::force_set_to_stdin();
    let ttyin = io::stdin();
    let mut ttyin = ttyin.lock();

    app.run(&mut ttyin, &mut ttyout, &buffer);

    Ok(())
}
