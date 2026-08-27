#![allow(nonstandard_style)]
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RuleItem<T> {
    User(T),
    Id,
    Str,
    Int,
    Float,
    Expr,
}
pub trait Token: Sized + Copy + 'static {
    const Id: RuleItem<Self> = RuleItem::Id;
    const String: RuleItem<Self> = RuleItem::Str;
    const Int: RuleItem<Self> = RuleItem::Int;
    const Float: RuleItem<Self> = RuleItem::Float;
    const Exer: RuleItem<Self> = RuleItem::Expr;
    fn assign_token() -> Vec<(Self, &'static [u8])>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenInstance<'a, T> {
    pub kind: RuleItem<T>,
    pub lexeme: &'a [u8],
    pub line: u32,
}
#[derive(Default)]
struct FxHasher(u64);

impl Hasher for FxHasher {
    #[inline(always)]
    fn write(&mut self, bytes: &[u8]) {
        const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
        let mut hash = self.0;
        for &b in bytes {
            hash = (hash.rotate_left(5) ^ b as u64).wrapping_mul(SEED);
        }
        self.0 = hash;
    }
    #[inline(always)]
    fn finish(&self) -> u64 {
        self.0
    }
}

type FxBuildHasher = BuildHasherDefault<FxHasher>;

pub struct Lexer<'a, T: Token> {
    source: &'a [u8],
    cursor_ptr: *const u8,
    end_ptr: *const u8,
    line: u32,
    symbol_keywords: [Option<T>; 256],
    word_keywords: WordKeywords<T>,
}

const WORD_KEYWORD_HASH_THRESHOLD: usize = 12;

enum WordKeywords<T> {
    Small(Vec<(&'static [u8], T)>),
    Hashed(HashMap<&'static [u8], T, FxBuildHasher>),
}

impl<T: Copy> WordKeywords<T> {
    fn build(entries: Vec<(&'static [u8], T)>) -> Self {
        if entries.len() <= WORD_KEYWORD_HASH_THRESHOLD {
            WordKeywords::Small(entries)
        } else {
            let mut map: HashMap<&'static [u8], T, FxBuildHasher> =
                HashMap::with_capacity_and_hasher(entries.len(), FxBuildHasher::default());
            map.extend(entries);
            WordKeywords::Hashed(map)
        }
    }

    #[inline(always)]
    fn get(&self, lexeme: &[u8]) -> Option<T> {
        match self {
            WordKeywords::Small(v) => v
                .iter()
                .find(|(pat, _)| *pat == lexeme)
                .map(|(_, kind)| *kind),
            WordKeywords::Hashed(m) => m.get(lexeme).copied(),
        }
    }
}

impl<'a, T: Token> Lexer<'a, T> {
    pub fn new(source: &'a [u8]) -> Self {
        let start_ptr = source.as_ptr();
        let end_ptr = unsafe { start_ptr.add(source.len()) };

        let mut symbol_keywords: [Option<T>; 256] = [None; 256];
        let mut word_entries: Vec<(&'static [u8], T)> = Vec::new();

        for (kind, pattern) in T::assign_token() {
            if let [byte] = pattern {
                symbol_keywords[*byte as usize] = Some(kind);
            } else {
                word_entries.push((pattern, kind));
            }
        }
        let word_keywords = WordKeywords::build(word_entries);

        Self {
            source,
            cursor_ptr: start_ptr,
            end_ptr,
            line: 0,
            symbol_keywords,
            word_keywords,
        }
    }

    pub fn scan_tokens(&mut self) -> Vec<TokenInstance<'a, T>> {
        let mut tokens = Vec::with_capacity(self.source.len() / 3);
        while !self.is_at_end() {
            self.skip_whitespace();
            if self.is_at_end() {
                break;
            }
            if let Some(token) = self.scan_next_token() {
                tokens.push(token);
            } else {
                self.adv();
            }
        }
        tokens
    }
    #[inline(always)]
    fn lookup_keyword(&self, lexeme: &[u8]) -> Option<T> {
        if let [byte] = lexeme {
            self.symbol_keywords[*byte as usize]
        } else {
            self.word_keywords.get(lexeme)
        }
    }

    #[inline]
    fn scan_next_token(&mut self) -> Option<TokenInstance<'a, T>> {
        let start = self.cursor_ptr;
        let c = self.peek();
        match c {
            b'"' => {
                self.adv();
                while !self.is_at_end() && self.peek() != b'"' {
                    if self.peek() == b'\n' {
                        self.line += 1;
                    }
                    self.adv();
                }
                if !self.is_at_end() {
                    self.adv();
                }
                Some(TokenInstance {
                    kind: RuleItem::Str,
                    lexeme: unsafe { self.make_lifetime_slice(start) },
                    line: self.line,
                })
            }
            b'0'..=b'9' => {
                while self.peek().is_ascii_digit() {
                    self.adv();
                }
                if self.peek() == b'.' && self.peek_next().is_ascii_digit() {
                    self.adv();
                    while self.peek().is_ascii_digit() {
                        self.adv();
                    }
                    Some(TokenInstance {
                        kind: RuleItem::Float,
                        lexeme: unsafe { self.make_lifetime_slice(start) },
                        line: self.line,
                    })
                } else {
                    Some(TokenInstance {
                        kind: RuleItem::Int,
                        lexeme: unsafe { self.make_lifetime_slice(start) },
                        line: self.line,
                    })
                }
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                self.adv();
                while self.peek().is_ascii_alphanumeric() || self.peek() == b'_' {
                    self.adv();
                }
                let lexeme = unsafe { self.make_lifetime_slice(start) };
                if let Some(user_kind) = self.lookup_keyword(lexeme) {
                    return Some(TokenInstance {
                        kind: RuleItem::User(user_kind),
                        lexeme,
                        line: self.line,
                    });
                }
                Some(TokenInstance {
                    kind: RuleItem::Id,
                    lexeme,
                    line: self.line,
                })
            }
            _ => {
                self.adv();
                let lexeme = unsafe { self.make_lifetime_slice(start) };
                if let Some(user_kind) = self.lookup_keyword(lexeme) {
                    return Some(TokenInstance {
                        kind: RuleItem::User(user_kind),
                        lexeme,
                        line: self.line,
                    });
                }
                None
            }
        }
    }
    #[inline]
    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                b' ' | b'\r' | b'\t' => {
                    self.adv();
                }
                b'\n' => {
                    self.line += 1;
                    self.adv();
                }
                _ => break,
            }
        }
    }
    #[inline(always)]
    fn is_at_end(&self) -> bool {
        self.cursor_ptr >= self.end_ptr
    }

    #[inline(always)]
    fn adv(&mut self) -> u8 {
        unsafe {
            let c = *self.cursor_ptr;
            self.cursor_ptr = self.cursor_ptr.add(1);
            c
        }
    }
    #[inline(always)]
    fn peek(&self) -> u8 {
        if self.is_at_end() {
            return b'\0';
        }
        unsafe { *self.cursor_ptr }
    }

    #[inline(always)]
    fn peek_next(&self) -> u8 {
        unsafe {
            let next = self.cursor_ptr.add(1);
            if next >= self.end_ptr { b'\0' } else { *next }
        }
    }

    #[inline(always)]
    unsafe fn make_lifetime_slice(&self, start: *const u8) -> &'a [u8] {
        let len = unsafe { self.cursor_ptr.offset_from(start) as usize };
        unsafe { std::slice::from_raw_parts(start, len) }
    }
}