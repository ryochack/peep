extern crate getopts;
extern crate termion;

use getopts::Options;
use std::env;
use std::io::{self, Write};

extern crate peep;
use peep::app::App;

fn print_usage(prog: &str, version: &str, opts: &Options) {
    let brief = format!(
        "{p} {v}\n\nUsage: {p} [OPTION]... [FILE]",
        p = prog,
        v = version
    );
    print!("{}", opts.usage(&brief));
}

fn print_version(prog: &str, version: &str) {
    println!("{} {}", prog, version);
}

fn build_app(prog: &str, version: &str, args: &[String]) -> (App, String) {
    use std::process;

    let mut opts = Options::new();
    opts.optopt("n", "lines", "set height of pane", "LINES")
        .optflag("N", "print-line-number", "print line numbers")
        .optflag("f", "follow", "output appended data as the file grows")
        .optflag("h", "help", "show this usage")
        .optflag("v", "version", "show version");

    let matches = match opts.parse(args) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    if matches.opt_present("h") {
        print_usage(prog, version, &opts);
        process::exit(0);
    }

    if matches.opt_present("v") {
        print_version(prog, version);
        process::exit(0);
    }

    let mut app: App = Default::default();
    app.show_linenumber = matches.opt_present("N");
    app.follow_mode = matches.opt_present("f");
    if let Ok(Some(nlines)) = matches.opt_get::<u16>("n") {
        app.nlines = nlines;
    }

    let file_path = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        if termion::is_tty(&io::stdin()) {
            // not find file name and pipe input
            writeln!(io::stderr(), "Missing filename (\"{} --help\" for help)", prog);
            process::exit(0);
        }
        "-".to_owned()
    };

    (app, file_path)
}

fn main() -> io::Result<()> {
    let prog = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let args: Vec<String> = env::args().collect();

    let (mut app, file_path) = build_app(prog, version, &args[1..]);

    app.run(&file_path)?;
    Ok(())
}
