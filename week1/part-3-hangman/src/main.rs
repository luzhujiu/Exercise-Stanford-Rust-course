// Simple Hangman Program
// User gets five incorrect guesses
// Word chosen randomly from words.txt
// Inspiration from: https://doc.rust-lang.org/book/ch02-00-guessing-game-tutorial.html
// This assignment will introduce you to some fundamental syntax in Rust:
// - variable declaration
// - string manipulation
// - conditional statements
// - loops
// - vectors
// - files
// - user input
// We've tried to limit/hide Rust's quirks since we'll discuss those details
// more in depth in the coming lectures.
extern crate rand;
use rand::Rng;
use std::fs;
use std::io;
use std::io::Write;

const NUM_INCORRECT_GUESSES: u32 = 5;
const WORDS_PATH: &str = "words.txt";

fn pick_a_random_word() -> String {
    let file_string = fs::read_to_string(WORDS_PATH).expect("Unable to read file.");
    let words: Vec<&str> = file_string.split('\n').collect();
    String::from(words[rand::thread_rng().gen_range(0, words.len())].trim())
}

fn main() {
    let secret_word = pick_a_random_word();
    // Note: given what you know about Rust so far, it's easier to pull characters out of a
    // vector than it is to pull them out of a string. You can get the ith character of
    // secret_word by doing secret_word_chars[i].
    let secret_word_chars: Vec<char> = secret_word.chars().collect();
    // Uncomment for debugging:
    println!("random word: {}", secret_word);

    // Your code here! :)
    let mut sofar = vec!['-'; secret_word.len()];
    let mut letters = vec![];
    let mut count = NUM_INCORRECT_GUESSES;

    while count > 0 {
        step(&mut sofar, &mut letters, &mut count, &secret_word_chars);
        if sofar == secret_word_chars {
            println!("Congratulations you guessed the secret word: {}!", secret_word);
            return;
        }
    }

    println!("Sorry, you ran out of guesses!");
}

fn step(sofar: &mut [char], letters: &mut Vec<char>, count: &mut u32, secret_word: &[char]) {
    println!("\nThe word so far is {}", sofar.iter().collect::<String>());
    println!("You have guessed the following letters: {}", letters.iter().collect::<String>());
    println!("You have {} guesses left", count);

    print!("Please guess a letter: ");
    io::stdout()
        .flush()
        .expect("Error flushing stdout.");

    let mut guess = String::new();

    io::stdin()
        .read_line(&mut guess)
        .expect("Error reading line.");

    let guess_chars: Vec<char> = guess.chars().collect();
    let guess_char = guess_chars[0];
    
    if secret_word.contains(&guess_char) {
        for (i, elem) in secret_word.iter().enumerate() {
            if *elem == guess_char {
                sofar[i] = guess_char;
            }
        }
        letters.push(guess_char);
        letters.sort();
    } else {
        println!("Sorry, that letter is not in the word");
        *count -= 1;
    }    
}
