use crossterm as ct;
use glint::figlet;
use std::fs::read_to_string;

pub fn main() {
    let figlet_src = read_to_string("src/big.flf").expect("src/big.flf must exist");
    let font = figlet::parse(figlet_src.lines()).expect("should be able to parse font");

    let mut output = font.create_vec();

    // println!("{:#?}", font);

    font.write_to_buf_color("feat", &mut output[..], |s| {
        ct::style(s).with(ct::Color::Red).to_string()
    });

    font.write_to_buf("(", &mut output[..]);

    font.write_to_buf_color("client", &mut output[..], |s| {
        ct::style(s).with(ct::Color::Blue).to_string()
    });

    font.write_to_buf(")", &mut output[..]);

    for line in output {
        println!("{}", line);
    }
}
