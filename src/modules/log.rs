const COLOR_CYAN: &str = "\x1b[36m";
const COLOR_RESET: &str = "\x1b[0m";

pub fn info(message: &str) {
    println!("{}{}{}", COLOR_CYAN, message, COLOR_RESET);
}
