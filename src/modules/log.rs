const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_BLUE: &str = "\x1b[34m";
const COLOR_CYAN: &str = "\x1b[36m";
const COLOR_BOLD: &str = "\x1b[1m";
const COLOR_RESET: &str = "\x1b[0m";

pub fn step(message: &str) {
    println!("{}{}==> {}{}", COLOR_CYAN, COLOR_BOLD, message, COLOR_RESET);
}

pub fn info(message: &str) {
    println!(
        "{}{}    => {}{}{}",
        COLOR_BLUE, COLOR_BOLD, COLOR_BLUE, message, COLOR_RESET
    );
}

pub fn success(message: &str) {
    println!(
        "{}{}    => {}{}{}",
        COLOR_GREEN, COLOR_BOLD, COLOR_GREEN, message, COLOR_RESET
    );
}
