// Leon Scott 
// Created: 8/28/2026

/*
Course of action: 
[x] general runtime prnntout title header and footer
[] exit runtime [q]... input processing
*/
use std::io;

pub fn main(){
    println!("------------------------------------------");
    println!("---------- LET THERE BE LIGHT ------------");
    println!("------------------------------------------\n\n");

    


    println!("\n\n------------------------------------------");
    println!("---------- AND            CUT ------------");
    println!("------------------------------------------\n\n");

    println!("[q] then ENTER to quit : ");
    let mut exit = String::new();
    io::stdin()
        .read_line(&mut exit)
        .expect("Failed to read line");

    if exit == "q" {
        std::process::exit;
    }

}