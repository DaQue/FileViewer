// Single-line comment
/*
 Multi-line block comment start
 Line 2 of comment
 */

fn main() {
    // Inline /* not a block */ still a single-line comment
    let x = 42; /* trailing block comment */
    let s = "string with // inside and /* not a comment */";
    /*
    Another block comment spanning
    multiple lines with /* nested marker */ that we ignore for highlighting
    */
    if x > 0 { println!("hello {}", x); }
}

