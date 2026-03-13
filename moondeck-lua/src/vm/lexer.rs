use super::error::VmError;

// ─── Token types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Integer(i64),
    Number(f64),
    String(String),
    Name(String),

    // Keywords
    And,
    Break,
    Do,
    Else,
    ElseIf,
    End,
    False,
    For,
    Function,
    If,
    In,
    Local,
    Nil,
    Not,
    Or,
    Return,
    Then,
    True,
    While,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    DotDot,
    Hash,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Assign,

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Dot,
    Colon,
    Comma,
    Semi,

    // Special
    Eof,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

// ─── Lexer ──────────────────────────────────────────────────────────────────

pub struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<SpannedToken>, VmError> {
        let mut tokens = Vec::new();
        loop {
            let st = self.next_token()?;
            let is_eof = st.token == Token::Eof;
            tokens.push(st);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<SpannedToken, VmError> {
        self.skip_whitespace_and_comments();

        let span = Span {
            line: self.line,
            col: self.col,
        };

        if self.pos >= self.source.len() {
            return Ok(SpannedToken {
                token: Token::Eof,
                span,
            });
        }

        let ch = self.source[self.pos];

        // String literals
        if ch == b'"' || ch == b'\'' {
            let s = self.read_string(ch)?;
            return Ok(SpannedToken {
                token: Token::String(s),
                span,
            });
        }

        // Numbers
        if ch.is_ascii_digit() || (ch == b'.' && self.peek_next().map_or(false, |c| c.is_ascii_digit())) {
            let tok = self.read_number()?;
            return Ok(SpannedToken { token: tok, span });
        }

        // Identifiers / keywords
        if ch.is_ascii_alphabetic() || ch == b'_' {
            let tok = self.read_name();
            return Ok(SpannedToken { token: tok, span });
        }

        // Operators and delimiters
        let token = match ch {
            b'+' => { self.advance(); Token::Plus }
            b'*' => { self.advance(); Token::Star }
            b'/' => { self.advance(); Token::Slash }
            b'%' => { self.advance(); Token::Percent }
            b'#' => { self.advance(); Token::Hash }
            b'(' => { self.advance(); Token::LParen }
            b')' => { self.advance(); Token::RParen }
            b'{' => { self.advance(); Token::LBrace }
            b'}' => { self.advance(); Token::RBrace }
            b'[' => { self.advance(); Token::LBracket }
            b']' => { self.advance(); Token::RBracket }
            b':' => { self.advance(); Token::Colon }
            b',' => { self.advance(); Token::Comma }
            b';' => { self.advance(); Token::Semi }

            b'-' => {
                self.advance();
                Token::Minus
            }

            b'.' => {
                self.advance();
                if self.peek() == Some(b'.') {
                    self.advance();
                    Token::DotDot
                } else {
                    Token::Dot
                }
            }

            b'=' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    Token::Eq
                } else {
                    Token::Assign
                }
            }

            b'~' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    Token::Ne
                } else {
                    return Err(self.compile_error("unexpected character '~'"));
                }
            }

            b'<' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    Token::Le
                } else {
                    Token::Lt
                }
            }

            b'>' => {
                self.advance();
                if self.peek() == Some(b'=') {
                    self.advance();
                    Token::Ge
                } else {
                    Token::Gt
                }
            }

            _ => {
                return Err(self.compile_error(&format!(
                    "unexpected character '{}'",
                    ch as char
                )));
            }
        };

        Ok(SpannedToken { token, span })
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(ch) = self.peek() {
                if ch == b' ' || ch == b'\t' || ch == b'\r' {
                    self.advance();
                } else if ch == b'\n' {
                    self.advance_newline();
                } else {
                    break;
                }
            }

            // Check for comments
            if self.pos + 1 < self.source.len()
                && self.source[self.pos] == b'-'
                && self.source[self.pos + 1] == b'-'
            {
                // Check for block comment --[[ ... ]]
                if self.pos + 3 < self.source.len()
                    && self.source[self.pos + 2] == b'['
                    && self.source[self.pos + 3] == b'['
                {
                    // Skip --[[
                    self.advance(); // -
                    self.advance(); // -
                    self.advance(); // [
                    self.advance(); // [
                    // Read until ]]
                    loop {
                        if self.pos >= self.source.len() {
                            break;
                        }
                        if self.source[self.pos] == b']'
                            && self.pos + 1 < self.source.len()
                            && self.source[self.pos + 1] == b']'
                        {
                            self.advance(); // ]
                            self.advance(); // ]
                            break;
                        }
                        if self.source[self.pos] == b'\n' {
                            self.advance_newline();
                        } else {
                            self.advance();
                        }
                    }
                } else {
                    // Single-line comment: skip to end of line
                    while let Some(ch) = self.peek() {
                        if ch == b'\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                continue;
            }

            break;
        }
    }

    fn read_string(&mut self, quote: u8) -> Result<String, VmError> {
        self.advance(); // skip opening quote
        let mut result = Vec::new();

        loop {
            if self.pos >= self.source.len() {
                return Err(self.compile_error("unterminated string"));
            }

            let ch = self.source[self.pos];

            if ch == quote {
                self.advance(); // skip closing quote
                break;
            }

            if ch == b'\n' {
                return Err(self.compile_error("unterminated string (newline)"));
            }

            if ch == b'\\' {
                self.advance();
                if self.pos >= self.source.len() {
                    return Err(self.compile_error("unterminated escape sequence"));
                }
                let esc = self.source[self.pos];
                let escaped = match esc {
                    b'\\' => b'\\',
                    b'n' => b'\n',
                    b't' => b'\t',
                    b'r' => b'\r',
                    b'"' => b'"',
                    b'\'' => b'\'',
                    _ => {
                        return Err(self.compile_error(&format!(
                            "invalid escape sequence '\\{}'",
                            esc as char
                        )));
                    }
                };
                result.push(escaped);
                self.advance();
            } else {
                result.push(ch);
                self.advance();
            }
        }

        String::from_utf8(result)
            .map_err(|_| self.compile_error("invalid UTF-8 in string literal"))
    }

    fn read_number(&mut self) -> Result<Token, VmError> {
        let start = self.pos;

        // Hex literal: 0x...
        if self.source[self.pos] == b'0'
            && self.pos + 1 < self.source.len()
            && (self.source[self.pos + 1] == b'x' || self.source[self.pos + 1] == b'X')
        {
            self.advance(); // '0'
            self.advance(); // 'x'

            if self.pos >= self.source.len() || !self.source[self.pos].is_ascii_hexdigit() {
                return Err(self.compile_error("expected hex digits after '0x'"));
            }

            while self.pos < self.source.len() && self.source[self.pos].is_ascii_hexdigit() {
                self.advance();
            }

            let hex_str = core::str::from_utf8(&self.source[start + 2..self.pos]).unwrap();
            let value = i64::from_str_radix(hex_str, 16)
                .map_err(|_| self.compile_error("invalid hex number"))?;
            return Ok(Token::Integer(value));
        }

        // Decimal integer or float
        let mut is_float = false;

        // Integer part (may be empty if starts with '.')
        while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
            self.advance();
        }

        // Fractional part
        if self.pos < self.source.len() && self.source[self.pos] == b'.' {
            // Make sure it's not `..` (concat operator)
            if self.pos + 1 < self.source.len() && self.source[self.pos + 1] == b'.' {
                // This is `..`, stop here and return the integer part
            } else {
                is_float = true;
                self.advance(); // '.'
                while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
                    self.advance();
                }
            }
        }

        // Exponent part
        if self.pos < self.source.len()
            && (self.source[self.pos] == b'e' || self.source[self.pos] == b'E')
        {
            is_float = true;
            self.advance(); // 'e'
            if self.pos < self.source.len()
                && (self.source[self.pos] == b'+' || self.source[self.pos] == b'-')
            {
                self.advance(); // sign
            }
            if self.pos >= self.source.len() || !self.source[self.pos].is_ascii_digit() {
                return Err(self.compile_error("expected digits in exponent"));
            }
            while self.pos < self.source.len() && self.source[self.pos].is_ascii_digit() {
                self.advance();
            }
        }

        let num_str = core::str::from_utf8(&self.source[start..self.pos]).unwrap();

        if is_float {
            let value: f64 = num_str
                .parse()
                .map_err(|_| self.compile_error("invalid number"))?;
            Ok(Token::Number(value))
        } else {
            let value: i64 = num_str
                .parse()
                .map_err(|_| self.compile_error("invalid integer"))?;
            Ok(Token::Integer(value))
        }
    }

    fn read_name(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.source.len()
            && (self.source[self.pos].is_ascii_alphanumeric() || self.source[self.pos] == b'_')
        {
            self.advance();
        }

        let name = core::str::from_utf8(&self.source[start..self.pos]).unwrap();

        match name {
            "and" => Token::And,
            "break" => Token::Break,
            "do" => Token::Do,
            "else" => Token::Else,
            "elseif" => Token::ElseIf,
            "end" => Token::End,
            "false" => Token::False,
            "for" => Token::For,
            "function" => Token::Function,
            "if" => Token::If,
            "in" => Token::In,
            "local" => Token::Local,
            "nil" => Token::Nil,
            "not" => Token::Not,
            "or" => Token::Or,
            "return" => Token::Return,
            "then" => Token::Then,
            "true" => Token::True,
            "while" => Token::While,
            _ => Token::Name(name.to_string()),
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────────

    fn peek(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.source.get(self.pos + 1).copied()
    }

    fn advance(&mut self) {
        if self.pos < self.source.len() {
            self.pos += 1;
            self.col += 1;
        }
    }

    fn advance_newline(&mut self) {
        self.pos += 1;
        self.line += 1;
        self.col = 1;
    }

    fn compile_error(&self, message: &str) -> VmError {
        VmError::Compile {
            message: message.to_string(),
            line: self.line,
            col: self.col,
        }
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(source: &str) -> Vec<Token> {
        Lexer::new(source)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|st| st.token)
            .collect()
    }

    fn lex_one(source: &str) -> Token {
        let tokens = lex(source);
        assert_eq!(tokens.len(), 2, "expected one token + Eof, got: {tokens:?}");
        tokens[0].clone()
    }

    // ── Literals ────────────────────────────────────────────────────────

    #[test]
    fn integer_decimal() {
        assert_eq!(lex_one("42"), Token::Integer(42));
        assert_eq!(lex_one("0"), Token::Integer(0));
        assert_eq!(lex_one("999"), Token::Integer(999));
    }

    #[test]
    fn integer_hex() {
        assert_eq!(lex_one("0xff"), Token::Integer(255));
        assert_eq!(lex_one("0XFF"), Token::Integer(255));
        assert_eq!(lex_one("0x1A"), Token::Integer(26));
    }

    #[test]
    fn float_basic() {
        assert_eq!(lex_one("3.14"), Token::Number(3.14));
        assert_eq!(lex_one("0.5"), Token::Number(0.5));
    }

    #[test]
    fn float_leading_dot() {
        assert_eq!(lex_one(".5"), Token::Number(0.5));
        assert_eq!(lex_one(".125"), Token::Number(0.125));
    }

    #[test]
    fn float_exponent() {
        assert_eq!(lex_one("1e10"), Token::Number(1e10));
        assert_eq!(lex_one("2.5e3"), Token::Number(2.5e3));
        assert_eq!(lex_one("1E-2"), Token::Number(0.01));
        assert_eq!(lex_one("5e+1"), Token::Number(50.0));
    }

    #[test]
    fn string_double_quote() {
        assert_eq!(
            lex_one(r#""hello""#),
            Token::String("hello".to_string())
        );
    }

    #[test]
    fn string_single_quote() {
        assert_eq!(
            lex_one("'world'"),
            Token::String("world".to_string())
        );
    }

    #[test]
    fn string_escapes() {
        assert_eq!(
            lex_one(r#""a\nb""#),
            Token::String("a\nb".to_string())
        );
        assert_eq!(
            lex_one(r#""a\tb""#),
            Token::String("a\tb".to_string())
        );
        assert_eq!(
            lex_one(r#""a\\b""#),
            Token::String("a\\b".to_string())
        );
        assert_eq!(
            lex_one(r#""a\"b""#),
            Token::String("a\"b".to_string())
        );
        assert_eq!(
            lex_one(r#""a\rb""#),
            Token::String("a\rb".to_string())
        );
        assert_eq!(
            lex_one("'a\\'b'"),
            Token::String("a'b".to_string())
        );
    }

    #[test]
    fn string_empty() {
        assert_eq!(lex_one(r#""""#), Token::String(String::new()));
        assert_eq!(lex_one("''"), Token::String(String::new()));
    }

    // ── Keywords ────────────────────────────────────────────────────────

    #[test]
    fn all_keywords() {
        let pairs = [
            ("and", Token::And),
            ("break", Token::Break),
            ("do", Token::Do),
            ("else", Token::Else),
            ("elseif", Token::ElseIf),
            ("end", Token::End),
            ("false", Token::False),
            ("for", Token::For),
            ("function", Token::Function),
            ("if", Token::If),
            ("in", Token::In),
            ("local", Token::Local),
            ("nil", Token::Nil),
            ("not", Token::Not),
            ("or", Token::Or),
            ("return", Token::Return),
            ("then", Token::Then),
            ("true", Token::True),
            ("while", Token::While),
        ];
        for (src, expected) in pairs {
            assert_eq!(lex_one(src), expected, "keyword: {src}");
        }
    }

    #[test]
    fn identifier_not_keyword() {
        assert_eq!(lex_one("foobar"), Token::Name("foobar".to_string()));
        assert_eq!(lex_one("_x"), Token::Name("_x".to_string()));
        assert_eq!(lex_one("while2"), Token::Name("while2".to_string()));
        assert_eq!(lex_one("IF"), Token::Name("IF".to_string()));
    }

    // ── Operators ───────────────────────────────────────────────────────

    #[test]
    fn single_char_operators() {
        assert_eq!(lex_one("+"), Token::Plus);
        assert_eq!(lex_one("-"), Token::Minus);
        assert_eq!(lex_one("*"), Token::Star);
        assert_eq!(lex_one("/"), Token::Slash);
        assert_eq!(lex_one("%"), Token::Percent);
        assert_eq!(lex_one("#"), Token::Hash);
    }

    #[test]
    fn assign_vs_eq() {
        assert_eq!(lex("= =="), vec![Token::Assign, Token::Eq, Token::Eof]);
    }

    #[test]
    fn less_vs_le() {
        assert_eq!(lex("< <="), vec![Token::Lt, Token::Le, Token::Eof]);
    }

    #[test]
    fn greater_vs_ge() {
        assert_eq!(lex("> >="), vec![Token::Gt, Token::Ge, Token::Eof]);
    }

    #[test]
    fn not_equal() {
        assert_eq!(lex_one("~="), Token::Ne);
    }

    #[test]
    fn dot_vs_dotdot() {
        assert_eq!(lex(". .."), vec![Token::Dot, Token::DotDot, Token::Eof]);
    }

    #[test]
    fn dotdot_in_context() {
        assert_eq!(
            lex("a..b"),
            vec![
                Token::Name("a".to_string()),
                Token::DotDot,
                Token::Name("b".to_string()),
                Token::Eof,
            ]
        );
    }

    // ── Delimiters ──────────────────────────────────────────────────────

    #[test]
    fn delimiters() {
        assert_eq!(
            lex("(){}[]:,;"),
            vec![
                Token::LParen, Token::RParen,
                Token::LBrace, Token::RBrace,
                Token::LBracket, Token::RBracket,
                Token::Colon, Token::Comma, Token::Semi,
                Token::Eof,
            ]
        );
    }

    // ── Comments ────────────────────────────────────────────────────────

    #[test]
    fn single_line_comment() {
        assert_eq!(lex("-- hello\n42"), vec![Token::Integer(42), Token::Eof]);
    }

    #[test]
    fn single_line_comment_at_eof() {
        assert_eq!(lex("42 -- end"), vec![Token::Integer(42), Token::Eof]);
    }

    #[test]
    fn block_comment() {
        assert_eq!(
            lex("--[[ block comment ]] 42"),
            vec![Token::Integer(42), Token::Eof]
        );
    }

    #[test]
    fn block_comment_multiline() {
        assert_eq!(
            lex("--[[\nline1\nline2\n]] 7"),
            vec![Token::Integer(7), Token::Eof]
        );
    }

    #[test]
    fn block_comment_preserves_line_count() {
        let tokens = Lexer::new("--[[\nline1\nline2\n]] x")
            .tokenize()
            .unwrap();
        // 'x' should be on line 4
        let x = &tokens[0];
        assert_eq!(x.token, Token::Name("x".to_string()));
        assert_eq!(x.span.line, 4);
    }

    // ── Line/col tracking ───────────────────────────────────────────────

    #[test]
    fn span_tracking() {
        let tokens = Lexer::new("local x = 10\nreturn x").tokenize().unwrap();
        // "local" at line 1, col 1
        assert_eq!(tokens[0].span.line, 1);
        assert_eq!(tokens[0].span.col, 1);
        // "x" at line 1, col 7
        assert_eq!(tokens[1].span.line, 1);
        assert_eq!(tokens[1].span.col, 7);
        // "return" at line 2, col 1
        assert_eq!(tokens[3].span.line, 2);
        assert_eq!(tokens[3].span.col, 1);
    }

    // ── Edge cases ──────────────────────────────────────────────────────

    #[test]
    fn empty_source() {
        assert_eq!(lex(""), vec![Token::Eof]);
    }

    #[test]
    fn only_whitespace() {
        assert_eq!(lex("   \n\t\n  "), vec![Token::Eof]);
    }

    #[test]
    fn minus_vs_comment() {
        assert_eq!(
            lex("a - b -- comment"),
            vec![
                Token::Name("a".to_string()),
                Token::Minus,
                Token::Name("b".to_string()),
                Token::Eof,
            ]
        );
    }

    #[test]
    fn number_before_dotdot() {
        // `1..x` should parse as Integer(1), DotDot, Name("x")
        assert_eq!(
            lex("1..x"),
            vec![
                Token::Integer(1),
                Token::DotDot,
                Token::Name("x".to_string()),
                Token::Eof,
            ]
        );
    }

    // ── Error cases ─────────────────────────────────────────────────────

    #[test]
    fn unterminated_string() {
        let result = Lexer::new("\"hello").tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn invalid_escape() {
        let result = Lexer::new(r#""\z""#).tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn lone_tilde() {
        let result = Lexer::new("~").tokenize();
        assert!(result.is_err());
    }

    // ── Mini Lua snippet ────────────────────────────────────────────────

    #[test]
    fn mini_lua_snippet() {
        let src = r#"
local function fib(n)
    if n <= 1 then
        return n
    end
    return fib(n - 1) + fib(n - 2)
end
"#;
        let tokens = lex(src);
        assert_eq!(
            tokens,
            vec![
                Token::Local,
                Token::Function,
                Token::Name("fib".to_string()),
                Token::LParen,
                Token::Name("n".to_string()),
                Token::RParen,
                Token::If,
                Token::Name("n".to_string()),
                Token::Le,
                Token::Integer(1),
                Token::Then,
                Token::Return,
                Token::Name("n".to_string()),
                Token::End,
                Token::Return,
                Token::Name("fib".to_string()),
                Token::LParen,
                Token::Name("n".to_string()),
                Token::Minus,
                Token::Integer(1),
                Token::RParen,
                Token::Plus,
                Token::Name("fib".to_string()),
                Token::LParen,
                Token::Name("n".to_string()),
                Token::Minus,
                Token::Integer(2),
                Token::RParen,
                Token::End,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn table_constructor_snippet() {
        let src = r#"local t = {1, "hello", x = true}"#;
        let tokens = lex(src);
        assert_eq!(
            tokens,
            vec![
                Token::Local,
                Token::Name("t".to_string()),
                Token::Assign,
                Token::LBrace,
                Token::Integer(1),
                Token::Comma,
                Token::String("hello".to_string()),
                Token::Comma,
                Token::Name("x".to_string()),
                Token::Assign,
                Token::True,
                Token::RBrace,
                Token::Eof,
            ]
        );
    }

    #[test]
    fn for_loop_snippet() {
        let src = "for i = 1, 10 do\n  x = x + i\nend";
        let tokens = lex(src);
        assert_eq!(
            tokens,
            vec![
                Token::For,
                Token::Name("i".to_string()),
                Token::Assign,
                Token::Integer(1),
                Token::Comma,
                Token::Integer(10),
                Token::Do,
                Token::Name("x".to_string()),
                Token::Assign,
                Token::Name("x".to_string()),
                Token::Plus,
                Token::Name("i".to_string()),
                Token::End,
                Token::Eof,
            ]
        );
    }
}
