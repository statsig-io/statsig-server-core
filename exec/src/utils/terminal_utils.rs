use colored::{Color, Colorize};

pub fn print_title(_emoji: &str, title: &str, color: Color) {
    println!("\n{}\n", format!("{}-- {} --", "", title).bold().color(color));
}
