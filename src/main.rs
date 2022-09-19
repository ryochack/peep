use getopts::Options;
use std::env;
use std::io;
use std::process;

use peep::app::App;

fn print_usage(prog: &str, version: &str, opts: &Options) {
    let brief = format!(
        "{p} {v}\n\nUsage: {p} [OPTION]... [FILE]",
        p = prog,
        v = version
    );
    println!("{}", opts.usage(&brief));
    println!(
        "Commands on Normal Mode:
    (num)j Ctrl-j Ctrl-n DownArrow CR LF
                        Scroll down
    (num)k Ctrl-k Ctrl-p UpArrow
                        Scroll up
    (num)d Ctrl-d       Scroll down half page
    (num)u Ctrl-u       Scroll up half page
    (num)f Ctrl-f PageDown SPACE
                        Scroll down a page
    (num)b Ctrl-b PageUp
                        Scroll up a page
    (num)l RightArrow   Scroll horizontally right
    (num)h LeftArrow    Scroll horizontally left
    (num)L              Scroll horizontally right half page
    (num)H              Scroll horizontally left half page
    0 Ctrl-a            Go to the beggining of line
    $ Ctrl-e            Go to the end of line
    g Home              Go to the beggining of file
    G End               Go to the end of file
    [num]g [num]G       Go to line [num]
    /pattern            Search forward in the file for the regex pattern
    n                   Search next
    N                   Search previous
    q Ctrl-c            Quit
    Q                   Quit with clearing pane
    (num)+              Increment screen height
    (num)-              Decrement screen height
    [num]=              Set screen height to [num]
    #                   Toggle line number printing
    !                   Toggle line wrapping
    ESC                 Cancel
    F                   Toggle to follow mode

Commands on Following Mode:
    /pattern            Highlight the regex pattern
    q Ctr-c             Quit
    Q                   Clear output and Quit
    (num)+              Increment screen height
    (num)-              Decrement screen height
    [num]=              Set screen height to [num]
    #                   Toggle line number printing
    !                   Toggle line wrapping
    ESC                 Cancel
    F                   Toggle to normal mode"
    );
}

fn print_version(prog: &str, version: &str) {
    println!("{} {}", prog, version);
}

fn run() -> io::Result<()> {
    let prog = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");
    let args: Vec<String> = env::args().skip(1).collect();

    let mut opts = Options::new();
    opts.optopt("n", "lines", "Set height of pane", "LINES")
        .optopt("s", "start", "Set start line of data at startup", "START")
        .optopt("t", "tab-width", "Set tab width", "WIDTH")
        .optflag("N", "print-line-number", "Print line numbers")
        .optflag("f", "follow", "Output appended data as the file grows")
        .optflag("w", "wrap", "Wrap text line")
        .optflag("h", "help", "Show this usage")
        .optflag("v", "version", "Show version");

    let matches = opts
        .parse(args)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;

    if matches.opt_present("h") {
        print_usage(prog, version, &opts);
        return Ok(());
    }

    if matches.opt_present("v") {
        print_version(prog, version);
        return Ok(());
    }

    let file_path = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        if termion::is_tty(&io::stdin()) {
            // not find file name and pipe input
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("missing filename (\"{} --help\" for help)", prog),
            ));
        }
        "-".to_owned()
    };

    let mut app: App = Default::default();
    app.show_linenumber = matches.opt_present("N");
    app.follow_mode = matches.opt_present("f");
    app.wraps_line = matches.opt_present("w");
    if let Ok(Some(nlines)) = matches.opt_get::<u16>("n") {
        app.nlines = nlines;
    }
    if let Ok(Some(tab_width)) = matches.opt_get::<u16>("t") {
        app.tab_width = tab_width;
    }
    if let Ok(Some(start_line)) = matches.opt_get::<u16>("s") {
        app.start_line = start_line;
    }

    app.run(&file_path)
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error. {}", e);
        process::exit(1);
    }
}
