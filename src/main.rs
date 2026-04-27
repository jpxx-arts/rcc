pub mod lexical_analyzer;
pub mod preprocessor;

fn main() {
    let tokens = lexical_analyzer::get_tokens("cl\na\ns\ns");
    println!("{:?}", tokens);
}
