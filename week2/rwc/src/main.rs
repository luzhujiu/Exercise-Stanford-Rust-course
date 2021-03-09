use std::env;
use std::process;
use std::io::{self, BufRead};
use std::fs::File;

fn read_file_lines(filename: &String) -> Result<Vec<String>, io::Error> {
    let file = File::open(filename)?;
    let mut vec = vec![];
    for line in io::BufReader::new(file).lines() {
        let line_str = line?;
        vec.push(line_str);
    }
    return Ok(vec);
}

fn num_of_words(lines: &Vec<String>) -> usize {
    lines.iter().map(|line| {
        line.trim().split(' ').count()
    }).sum()
}

fn num_of_characters(lines: &Vec<String>) -> usize {
    lines.iter().map(|line| {
        line.trim().chars().count()
    }).sum()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Too few arguments.");
        process::exit(1);
    }
    let filename = &args[1];
    let lines = read_file_lines(filename).expect("file not open");
    
    println!("num of lines = {}", lines.len());

    let num_words = num_of_words(&lines);
    println!("num of words = {}", num_words);
    
    let num_characters = num_of_characters(&lines);
    println!("num of characters = {}", num_characters);
}
