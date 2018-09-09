#[allow(dead_code)]
pub fn log(msg: &str) {
    const LOG_PATH: &str = "./peep.log";
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut w = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_PATH)
        .unwrap();
    let _ = writeln!(&mut w, "{}", msg);
}
