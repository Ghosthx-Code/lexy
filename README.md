# Lexy 
A high speed lexer lib

## Uses
```rust
#[derive(Debug, Clone, Copy)]
enum TokenKind {
  Print,
  SemiColon,
}
impl lexy::Token for TokenKind {
  fn assign_tokens() -> Vec<(Self, &'static [u8])> {
    vec![
      (TokenKind::Print, b"print"),
      (TokenKind::SemiColon, b";"),
    ]
  }
}

fn main() {
  let source = b"
    print \"Hello lexy\";
  ";
  let tokens = lexy::Lexer::<TokenKind>::new(source).scan_tokens();
  println!("{:#?}", tokens);
}
```
Strings, Id, Int, Float, Char, are auto lexed
