/// Particleex command compiler.
///
/// Ports the JavaScript Particleex → NBL pipeline to Rust:
///   1. Expression tokenizer + recursive-descent parser + tree-walking evaluator
///   2. Particleex command parser (normal, parameter, polarparameter, tick*, rgba*)
///   3. Track generator  →  Vec<Vec<Particle>>  (frame snapshots)
use crate::player::Particle;
use rand::Rng;
use std::collections::HashMap;
use std::f64::consts::{E, PI};

// ─────────────────────── Constants ───────────────────────

const TIME_SCALE: f64 = 3.0;

// ─────────────────────── Expression Language ───────────────────────

#[derive(Debug, Clone)]
enum Token {
    Num(f64),
    Ident(String),
    Op(char), // single-char operator  + - * / % ^ , ; ( )
    Eq,       // ==
    Ne,       // !=
    Le,       // <=
    Ge,       // >=
    And,      // &&
    Or,       // ||
    Assign,   // = (single)
    Lt,       // <
    Gt,       // >
}

fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<char> = src.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < len {
        let c = chars[i];
        // Skip whitespace
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        // Number
        if c.is_ascii_digit() || (c == '.' && i + 1 < len && chars[i + 1].is_ascii_digit()) {
            let start = i;
            while i < len && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            // scientific notation
            if i < len && (chars[i] == 'e' || chars[i] == 'E') {
                i += 1;
                if i < len && (chars[i] == '+' || chars[i] == '-') {
                    i += 1;
                }
                while i < len && chars[i].is_ascii_digit() {
                    i += 1;
                }
            }
            let s: String = chars[start..i].iter().collect();
            tokens.push(Token::Num(s.parse::<f64>().unwrap_or(0.0)));
            continue;
        }
        // Identifier
        if c.is_ascii_alphabetic() || c == '_' || c == '$' {
            let start = i;
            while i < len
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '$')
            {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            tokens.push(Token::Ident(s));
            continue;
        }
        // Two-char operators
        if i + 1 < len {
            let c2 = chars[i + 1];
            match (c, c2) {
                ('=', '=') => {
                    tokens.push(Token::Eq);
                    i += 2;
                    continue;
                }
                ('!', '=') => {
                    tokens.push(Token::Ne);
                    i += 2;
                    continue;
                }
                ('<', '=') => {
                    tokens.push(Token::Le);
                    i += 2;
                    continue;
                }
                ('>', '=') => {
                    tokens.push(Token::Ge);
                    i += 2;
                    continue;
                }
                ('&', '&') => {
                    tokens.push(Token::And);
                    i += 2;
                    continue;
                }
                ('|', '|') => {
                    tokens.push(Token::Or);
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        // Single-char
        match c {
            '=' => {
                tokens.push(Token::Assign);
                i += 1;
            }
            '<' => {
                tokens.push(Token::Lt);
                i += 1;
            }
            '>' => {
                tokens.push(Token::Gt);
                i += 1;
            }
            '+' | '-' | '*' | '/' | '%' | '^' | '(' | ')' | ',' | ';' | '!' => {
                tokens.push(Token::Op(c));
                i += 1;
            }
            _ => {
                i += 1;
            } // skip unknown
        }
    }
    tokens
}

// ─── AST ───

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Var(String),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryNeg(Box<Expr>),
    UnaryNot(Box<Expr>),
    Call(String, Vec<Expr>),
    #[allow(dead_code)]
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy)]
enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

// ─── Recursive-descent parser ───

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(t)
        } else {
            None
        }
    }

    fn eat_op(&mut self, ch: char) -> bool {
        if let Some(Token::Op(c)) = self.peek() {
            if *c == ch {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    /// Parse a full expression (lowest precedence = logical OR)
    fn parse_expr(&mut self) -> Expr {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Expr {
        let mut left = self.parse_and();
        while matches!(self.peek(), Some(Token::Or)) {
            self.advance();
            let right = self.parse_and();
            left = Expr::BinOp(Box::new(left), BinOp::Or, Box::new(right));
        }
        left
    }

    fn parse_and(&mut self) -> Expr {
        let mut left = self.parse_comparison();
        while matches!(self.peek(), Some(Token::And)) {
            self.advance();
            let right = self.parse_comparison();
            left = Expr::BinOp(Box::new(left), BinOp::And, Box::new(right));
        }
        left
    }

    fn parse_comparison(&mut self) -> Expr {
        let mut left = self.parse_additive();
        loop {
            let op = match self.peek() {
                Some(Token::Eq) => BinOp::Eq,
                Some(Token::Ne) => BinOp::Ne,
                Some(Token::Lt) => BinOp::Lt,
                Some(Token::Gt) => BinOp::Gt,
                Some(Token::Le) => BinOp::Le,
                Some(Token::Ge) => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive();
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        left
    }

    fn parse_additive(&mut self) -> Expr {
        let mut left = self.parse_multiplicative();
        loop {
            if self.matches_op('+') {
                self.advance();
                let right = self.parse_multiplicative();
                left = Expr::BinOp(Box::new(left), BinOp::Add, Box::new(right));
            } else if self.matches_op('-') {
                self.advance();
                let right = self.parse_multiplicative();
                left = Expr::BinOp(Box::new(left), BinOp::Sub, Box::new(right));
            } else {
                break;
            }
        }
        left
    }

    fn parse_multiplicative(&mut self) -> Expr {
        let mut left = self.parse_power();
        loop {
            if self.matches_op('*') {
                self.advance();
                let right = self.parse_power();
                left = Expr::BinOp(Box::new(left), BinOp::Mul, Box::new(right));
            } else if self.matches_op('/') {
                self.advance();
                let right = self.parse_power();
                left = Expr::BinOp(Box::new(left), BinOp::Div, Box::new(right));
            } else if self.matches_op('%') {
                self.advance();
                let right = self.parse_power();
                left = Expr::BinOp(Box::new(left), BinOp::Mod, Box::new(right));
            } else {
                break;
            }
        }
        left
    }

    fn parse_power(&mut self) -> Expr {
        let base = self.parse_unary();
        if self.matches_op('^') {
            self.advance();
            let exp = self.parse_unary(); // right-assoc
            Expr::BinOp(Box::new(base), BinOp::Pow, Box::new(exp))
        } else {
            base
        }
    }

    fn parse_unary(&mut self) -> Expr {
        if self.matches_op('-') {
            self.advance();
            let expr = self.parse_unary();
            return Expr::UnaryNeg(Box::new(expr));
        }
        if self.matches_op('!') {
            self.advance();
            let expr = self.parse_unary();
            return Expr::UnaryNot(Box::new(expr));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Expr {
        match self.peek().cloned() {
            Some(Token::Num(n)) => {
                self.advance();
                Expr::Num(n)
            }
            Some(Token::Ident(name)) => {
                self.advance();
                // Function call?  name(...)
                if self.matches_op('(') {
                    self.advance(); // eat '('
                    let mut args = Vec::new();
                    if !self.matches_op(')') {
                        args.push(self.parse_expr());
                        while self.matches_op(',') {
                            self.advance();
                            args.push(self.parse_expr());
                        }
                    }
                    self.eat_op(')');
                    Expr::Call(name, args)
                } else {
                    Expr::Var(name)
                }
            }
            Some(Token::Op('(')) => {
                self.advance();
                let expr = self.parse_expr();
                self.eat_op(')');
                expr
            }
            _ => {
                // Fallback: return 0
                Expr::Num(0.0)
            }
        }
    }

    fn matches_op(&self, ch: char) -> bool {
        matches!(self.peek(), Some(Token::Op(c)) if *c == ch)
    }
}

// ─── Statement-level parser ───

#[derive(Debug, Clone)]
enum Stmt {
    Assign(String, Expr),
    MultiAssign(Vec<String>, Vec<Expr>),
    ExprStmt(Expr),
}

fn parse_statements(tokens: Vec<Token>) -> Vec<Stmt> {
    // Split by semicolons first
    let mut groups: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    for t in tokens {
        if matches!(t, Token::Op(';')) {
            if !current.is_empty() {
                groups.push(std::mem::take(&mut current));
            }
        } else {
            current.push(t);
        }
    }
    if !current.is_empty() {
        groups.push(current);
    }

    let mut stmts = Vec::new();
    for group in groups {
        // Find top-level '=' (not inside parens)
        let mut assign_idx = None;
        let mut depth = 0;
        for (i, t) in group.iter().enumerate() {
            match t {
                Token::Op('(') => depth += 1,
                Token::Op(')') => depth -= 1,
                Token::Assign if depth == 0 => {
                    assign_idx = Some(i);
                    break;
                }
                _ => {}
            }
        }

        if let Some(ai) = assign_idx {
            let lhs_tokens: Vec<Token> = group[..ai].to_vec();
            let rhs_tokens: Vec<Token> = group[ai + 1..].to_vec();

            // Check if LHS is multi-assignment  (a, b = ...)
            let mut lhs_names = Vec::new();
            let mut has_comma = false;
            for t in &lhs_tokens {
                match t {
                    Token::Ident(name) => lhs_names.push(name.clone()),
                    Token::Op(',') => has_comma = true,
                    _ => {}
                }
            }

            let mut rhs_parser = Parser::new(rhs_tokens);

            if has_comma && lhs_names.len() > 1 {
                // Multi-assign: a, b, c = expr1, expr2, expr3
                let mut rhs_exprs = vec![rhs_parser.parse_expr()];
                while rhs_parser.matches_op(',') {
                    rhs_parser.advance();
                    rhs_exprs.push(rhs_parser.parse_expr());
                }
                stmts.push(Stmt::MultiAssign(lhs_names, rhs_exprs));
            } else if lhs_names.len() == 1 {
                let rhs = rhs_parser.parse_expr();
                stmts.push(Stmt::Assign(lhs_names[0].clone(), rhs));
            } else {
                // Fallback: expression
                let mut p = Parser::new(group);
                stmts.push(Stmt::ExprStmt(p.parse_expr()));
            }
        } else {
            let mut p = Parser::new(group);
            stmts.push(Stmt::ExprStmt(p.parse_expr()));
        }
    }
    stmts
}

// ─── Evaluator context ───

#[derive(Clone)]
pub struct ExprContext {
    pub vars: HashMap<String, f64>,
}

impl ExprContext {
    fn new() -> Self {
        let mut vars = HashMap::new();
        vars.insert("PI".into(), PI);
        vars.insert("E".into(), E);
        Self { vars }
    }

    fn get(&self, name: &str) -> f64 {
        self.vars.get(name).copied().unwrap_or(0.0)
    }

    fn set(&mut self, name: &str, val: f64) {
        self.vars.insert(name.to_string(), val);
    }
}

fn eval_expr(expr: &Expr, ctx: &mut ExprContext) -> f64 {
    match expr {
        Expr::Num(n) => *n,
        Expr::Var(name) => ctx.get(name),
        Expr::BinOp(l, op, r) => {
            let lv = eval_expr(l, ctx);
            let rv = eval_expr(r, ctx);
            match op {
                BinOp::Add => lv + rv,
                BinOp::Sub => lv - rv,
                BinOp::Mul => lv * rv,
                BinOp::Div => {
                    if rv.abs() < 1e-15 {
                        0.0
                    } else {
                        lv / rv
                    }
                }
                BinOp::Mod => {
                    if rv.abs() < 1e-15 {
                        0.0
                    } else {
                        lv % rv
                    }
                }
                BinOp::Pow => lv.powf(rv),
                BinOp::Eq => {
                    if (lv - rv).abs() < 1e-10 {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Ne => {
                    if (lv - rv).abs() >= 1e-10 {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Lt => {
                    if lv < rv {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Gt => {
                    if lv > rv {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Le => {
                    if lv <= rv {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Ge => {
                    if lv >= rv {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::And => {
                    if lv != 0.0 && rv != 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
                BinOp::Or => {
                    if lv != 0.0 || rv != 0.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
            }
        }
        Expr::UnaryNeg(e) => -eval_expr(e, ctx),
        Expr::UnaryNot(e) => {
            if eval_expr(e, ctx) == 0.0 {
                1.0
            } else {
                0.0
            }
        }
        Expr::Call(name, args) => {
            let mut rng = rand::thread_rng();
            let a: Vec<f64> = args.iter().map(|e| eval_expr(e, ctx)).collect();
            match name.as_str() {
                "sin" => a.first().copied().unwrap_or(0.0).sin(),
                "cos" => a.first().copied().unwrap_or(0.0).cos(),
                "tan" => a.first().copied().unwrap_or(0.0).tan(),
                "asin" => a.first().copied().unwrap_or(0.0).asin(),
                "acos" => a.first().copied().unwrap_or(0.0).acos(),
                "atan" => a.first().copied().unwrap_or(0.0).atan(),
                "atan2" => {
                    let y = a.first().copied().unwrap_or(0.0);
                    let x = a.get(1).copied().unwrap_or(0.0);
                    y.atan2(x)
                }
                "sinh" => a.first().copied().unwrap_or(0.0).sinh(),
                "cosh" => a.first().copied().unwrap_or(0.0).cosh(),
                "tanh" => a.first().copied().unwrap_or(0.0).tanh(),
                "exp" => a.first().copied().unwrap_or(0.0).exp(),
                "log" | "ln" => a.first().copied().unwrap_or(0.0).ln(),
                "log10" => a.first().copied().unwrap_or(0.0).log10(),
                "expm1" => a.first().copied().unwrap_or(0.0).exp_m1(),
                "log1p" => a.first().copied().unwrap_or(0.0).ln_1p(),
                "pow" => {
                    let base = a.first().copied().unwrap_or(0.0);
                    let exp = a.get(1).copied().unwrap_or(0.0);
                    base.powf(exp)
                }
                "sqrt" => a.first().copied().unwrap_or(0.0).sqrt(),
                "cbrt" => a.first().copied().unwrap_or(0.0).cbrt(),
                "hypot" => {
                    let x = a.first().copied().unwrap_or(0.0);
                    let y = a.get(1).copied().unwrap_or(0.0);
                    x.hypot(y)
                }
                "ceil" => a.first().copied().unwrap_or(0.0).ceil(),
                "floor" => a.first().copied().unwrap_or(0.0).floor(),
                "round" | "rint" => a.first().copied().unwrap_or(0.0).round(),
                "max" => {
                    let x = a.first().copied().unwrap_or(0.0);
                    let y = a.get(1).copied().unwrap_or(0.0);
                    x.max(y)
                }
                "min" => {
                    let x = a.first().copied().unwrap_or(0.0);
                    let y = a.get(1).copied().unwrap_or(0.0);
                    x.min(y)
                }
                "abs" | "signum" => {
                    let v = a.first().copied().unwrap_or(0.0);
                    if name == "abs" {
                        v.abs()
                    } else {
                        v.signum()
                    }
                }
                "random" => rng.gen::<f64>(),
                "toRadians" => a.first().copied().unwrap_or(0.0).to_radians(),
                "toDegrees" => a.first().copied().unwrap_or(0.0).to_degrees(),
                // Exact arithmetic (just regular math in f64, no overflow check needed)
                "addExact" => {
                    let x = a.first().copied().unwrap_or(0.0);
                    let y = a.get(1).copied().unwrap_or(0.0);
                    x + y
                }
                "nextUp" => {
                    let v = a.first().copied().unwrap_or(0.0);
                    f64::from_bits(v.to_bits() + 1)
                }
                "nextDown" => {
                    let v = a.first().copied().unwrap_or(0.0);
                    f64::from_bits(v.to_bits() - 1)
                }
                // Bit scaling
                "scalb" => {
                    let d = a.first().copied().unwrap_or(0.0);
                    let sf = a.get(1).copied().unwrap_or(0.0) as i32;
                    d * 2.0_f64.powi(sf)
                }
                _ => 0.0,
            }
        }
        Expr::Conditional(cond, then, else_) => {
            if eval_expr(cond, ctx) != 0.0 {
                eval_expr(then, ctx)
            } else {
                eval_expr(else_, ctx)
            }
        }
    }
}

fn exec_stmts(stmts: &[Stmt], ctx: &mut ExprContext) {
    for stmt in stmts {
        match stmt {
            Stmt::Assign(name, expr) => {
                let val = eval_expr(expr, ctx);
                ctx.set(name, val);
            }
            Stmt::MultiAssign(names, exprs) => {
                let vals: Vec<f64> = exprs.iter().map(|e| eval_expr(e, ctx)).collect();
                for (i, name) in names.iter().enumerate() {
                    let val = vals.get(i).copied().unwrap_or(0.0);
                    ctx.set(name, val);
                }
            }
            Stmt::ExprStmt(expr) => {
                let _ = eval_expr(expr, ctx);
            }
        }
    }
}

/// Compile an expression string into executable statements. Returns None if empty.
fn compile_expr(src: &str) -> Option<Vec<Stmt>> {
    let src = src.trim();
    if src.is_empty() || src == "null" {
        return None;
    }
    let tokens = tokenize(src);
    if tokens.is_empty() {
        return None;
    }
    Some(parse_statements(tokens))
}

// ─────────────────────── Command Types ───────────────────────

#[derive(Debug, Clone)]
struct CommandConfig {
    has_color: bool,
    is_normal: bool,
    is_conditional: bool,
    is_animated: bool,
    is_polar: bool,
}

/// Normalize hyphened command names → joined form, e.g. "tick-parameter" → "tickparameter"
fn normalize_command_name(name: &str) -> String {
    name.replace('-', "")
}

fn get_command_config(type_name: &str) -> Option<CommandConfig> {
    let name = normalize_command_name(type_name);
    match name.as_str() {
        "normal" => Some(CommandConfig {
            has_color: true,
            is_normal: true,
            is_conditional: false,
            is_animated: false,
            is_polar: false,
        }),
        "conditional" => Some(CommandConfig {
            has_color: true,
            is_normal: false,
            is_conditional: true,
            is_animated: false,
            is_polar: false,
        }),
        "parameter" => Some(CommandConfig {
            has_color: true,
            is_normal: false,
            is_conditional: false,
            is_animated: false,
            is_polar: false,
        }),
        "polarparameter" => Some(CommandConfig {
            has_color: true,
            is_normal: false,
            is_conditional: false,
            is_animated: false,
            is_polar: true,
        }),
        "rgbaparameter" => Some(CommandConfig {
            has_color: false,
            is_normal: false,
            is_conditional: false,
            is_animated: false,
            is_polar: false,
        }),
        "rgbapolarparameter" => Some(CommandConfig {
            has_color: false,
            is_normal: false,
            is_conditional: false,
            is_animated: false,
            is_polar: true,
        }),
        "tickparameter" => Some(CommandConfig {
            has_color: true,
            is_normal: false,
            is_conditional: false,
            is_animated: true,
            is_polar: false,
        }),
        "tickpolarparameter" => Some(CommandConfig {
            has_color: true,
            is_normal: false,
            is_conditional: false,
            is_animated: true,
            is_polar: true,
        }),
        "rgbatickparameter" => Some(CommandConfig {
            has_color: false,
            is_normal: false,
            is_conditional: false,
            is_animated: true,
            is_polar: false,
        }),
        "rgbatickpolarparameter" => Some(CommandConfig {
            has_color: false,
            is_normal: false,
            is_conditional: false,
            is_animated: true,
            is_polar: true,
        }),
        _ => None,
    }
}

// ─────────────────────── Command Parsing ───────────────────────

#[derive(Debug, Clone)]
struct ParsedCommand {
    #[allow(dead_code)]
    type_name: String,
    config: CommandConfig,
    center: [f64; 3],
    color: [f64; 4],
    base_velocity: [f64; 3],
    // Normal / Conditional mode
    range: [f64; 3],
    count: u32,
    lifespan: u32,
    speed_expr: Option<String>,
    // Parameter mode
    t_begin: f64,
    t_end: f64,
    t_step: f64,
    shape_expr: Option<String>,
    cpt: u32, // count per tick (animated)
    #[allow(dead_code)]
    speed_step: f64,
}

fn parse_coord(val: &str) -> f64 {
    let s = val.replace('~', "");
    if s.is_empty() {
        0.0
    } else {
        s.parse::<f64>().unwrap_or(0.0)
    }
}

fn parse_num(val: &str, default: f64) -> f64 {
    val.parse::<f64>().unwrap_or(default)
}

/// Parse lifespan: 0 → default (200), -1 → very long (6000), positive → as-is
fn resolve_lifespan(raw: &str, default: u32) -> u32 {
    match raw.parse::<i32>() {
        Ok(n) if n < 0 => 6000, // -1 means "infinite", cap at ~5 min
        Ok(0) => default,       // 0 means "use default lifetime"
        Ok(n) => n as u32,
        Err(_) => default,
    }
}

fn parse_int(val: &str, default: u32) -> u32 {
    val.parse::<u32>().unwrap_or(default)
}

fn split_args(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    for ch in line.chars() {
        if ch == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            continue;
        }
        if ch == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            continue;
        }
        if ch.is_ascii_whitespace() && !in_single_quote && !in_double_quote {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}

fn parse_command(line: &str) -> Option<ParsedCommand> {
    let parts = split_args(line);
    if parts.len() < 3 {
        return None;
    }

    let mut idx = 0;
    // Skip leading command word (particleex / particlex / /particleex / /particlex)
    let first_lower = parts[0].to_lowercase();
    if first_lower.contains("particlex") || first_lower.contains("particleex") {
        idx = 1;
    }

    let type_name = parts.get(idx)?.to_lowercase();
    let config = get_command_config(&type_name)?;
    idx += 1;

    let _particle_name = parts.get(idx).cloned().unwrap_or_default();
    idx += 1;

    let cx = parse_coord(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"));
    idx += 1;
    let cy = parse_coord(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"));
    idx += 1;
    let cz = parse_coord(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"));
    idx += 1;

    let (cr, cg, cb, ca) = if config.has_color {
        let r = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
        idx += 1;
        let g = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
        idx += 1;
        let b = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
        idx += 1;
        let a = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
        idx += 1;
        (r, g, b, a)
    } else {
        (1.0, 1.0, 1.0, 1.0)
    };

    let base_vx = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
    idx += 1;
    let base_vy = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
    idx += 1;
    let base_vz = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
    idx += 1;

    if config.is_normal || config.is_conditional {
        let range_x = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
        idx += 1;
        let range_y = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
        idx += 1;
        let range_z = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0"), 0.0);
        idx += 1;

        if config.is_conditional {
            // conditional: <range> <condition_expr> [step] [lifespan] [speed_expr] [speed_step] [group]
            let cond_expr = parts.get(idx).cloned();
            idx += 1;
            let t_step = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0.5"), 0.5)
                .abs()
                .max(0.01);
            idx += 1;
            let lifespan =
                resolve_lifespan(parts.get(idx).map(|s| s.as_str()).unwrap_or("200"), 200);
            idx += 1;
            let speed_expr = parts.get(idx).cloned();

            return Some(ParsedCommand {
                type_name,
                config,
                center: [cx, cy, cz],
                color: [cr, cg, cb, ca],
                base_velocity: [base_vx, base_vy, base_vz],
                range: [range_x, range_y, range_z],
                count: 0,
                lifespan,
                speed_expr,
                t_begin: 0.0,
                t_end: 0.0,
                t_step,
                shape_expr: cond_expr,
                cpt: 0,
                speed_step: 1.0,
            });
        }

        // normal mode
        let count = parse_int(parts.get(idx).map(|s| s.as_str()).unwrap_or("1"), 1).max(1);
        idx += 1;

        let lifespan_str = parts.get(idx).map(|s| s.as_str()).unwrap_or("200");
        let mut lifespan = resolve_lifespan(lifespan_str, 200);
        idx += 1;
        let mut speed_expr: Option<String> = None;

        while idx < parts.len() {
            let arg = &parts[idx];
            idx += 1;
            if arg.contains('=') || arg.contains(';') {
                speed_expr = Some(arg.clone());
            }
        }

        if lifespan == 0 {
            lifespan = 200;
        }

        return Some(ParsedCommand {
            type_name,
            config,
            center: [cx, cy, cz],
            color: [cr, cg, cb, ca],
            base_velocity: [base_vx, base_vy, base_vz],
            range: [range_x, range_y, range_z],
            count,
            lifespan,
            speed_expr,
            t_begin: 0.0,
            t_end: 0.0,
            t_step: 0.0,
            shape_expr: None,
            cpt: 0,
            speed_step: 1.0,
        });
    }

    // Parameter modes
    let t_begin = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("-10"), -10.0);
    idx += 1;
    let t_end = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("10"), 10.0);
    idx += 1;
    let shape_expr = parts.get(idx).cloned();
    idx += 1;
    let mut t_step = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("0.1"), 0.1);
    idx += 1;
    if t_step.abs() < 0.000001 {
        t_step = 0.1;
    }

    let cpt = if config.is_animated {
        let v = parse_int(parts.get(idx).map(|s| s.as_str()).unwrap_or("10"), 10);
        idx += 1;
        if v == 0 {
            10
        } else {
            v
        }
    } else {
        10
    };

    let lifespan = resolve_lifespan(parts.get(idx).map(|s| s.as_str()).unwrap_or("200"), 200);
    idx += 1;
    let speed_expr = parts.get(idx).cloned();
    idx += 1;
    let speed_step = parse_num(parts.get(idx).map(|s| s.as_str()).unwrap_or("1"), 1.0);

    Some(ParsedCommand {
        type_name,
        config,
        center: [cx, cy, cz],
        color: [cr, cg, cb, ca],
        base_velocity: [base_vx, base_vy, base_vz],
        range: [0.0; 3],
        count: 0,
        lifespan,
        speed_expr,
        t_begin,
        t_end,
        t_step,
        shape_expr,
        cpt,
        speed_step,
    })
}

// ─────────────────────── Track ───────────────────────

#[derive(Debug, Clone)]
struct Keyframe {
    tick: u32,
    x: f64,
    y: f64,
    z: f64,
    r: u8,
    g: u8,
    b: u8,
    a: u8,
    size: f64,
}

struct Track {
    id: i32,
    keyframes: Vec<Keyframe>,
}

// ─────────────────────── Track Generation ───────────────────────

fn generate_tracks(cmd: &ParsedCommand, start_id: i32) -> (Vec<Track>, i32) {
    let mut tracks = Vec::new();
    let mut current_id = start_id;
    let mut rng = rand::thread_rng();

    // ─── Conditional mode: 3D range-based generation ───
    if cmd.config.is_conditional {
        let cond_stmts = cmd.shape_expr.as_deref().and_then(compile_expr);
        let speed_stmts = cmd.speed_expr.as_deref().and_then(compile_expr);
        let step = cmd.t_step.abs().max(0.01);

        let mut cx = -cmd.range[0];
        while cx <= cmd.range[0] + 0.0001 {
            let mut cy = -cmd.range[1];
            while cy <= cmd.range[1] + 0.0001 {
                let mut cz = -cmd.range[2];
                while cz <= cmd.range[2] + 0.0001 {
                    let mut ctx = ExprContext::new();
                    ctx.set("x", cx);
                    ctx.set("y", cy);
                    ctx.set("z", cz);
                    ctx.set("s1", 0.0);
                    ctx.set("s2", 0.0);
                    ctx.set("dis", (cx * cx + cy * cy + cz * cz).sqrt());
                    ctx.set("cr", cmd.color[0]);
                    ctx.set("cg", cmd.color[1]);
                    ctx.set("cb", cmd.color[2]);
                    ctx.set("alpha", cmd.color[3]);

                    // Evaluate condition: result ≠ 0 means spawn
                    let spawn = if let Some(ref stmts) = cond_stmts {
                        exec_stmts(stmts, &mut ctx);
                        // The last assignment or expression result
                        // Check if any output changed or if condition is true
                        // In particlex, the condition returns the last statement value
                        true // If stmts execute without setting destory=1, spawn
                    } else {
                        true
                    };

                    if spawn && ctx.get("destory") == 0.0 {
                        let mut track = Track {
                            id: current_id,
                            keyframes: Vec::new(),
                        };
                        current_id += 1;

                        let mut cur_x = cmd.center[0] + cx;
                        let mut cur_y = cmd.center[1] + cy;
                        let mut cur_z = cmd.center[2] + cz;
                        let mut cur_vx = cmd.base_velocity[0];
                        let mut cur_vy = cmd.base_velocity[1];
                        let mut cur_vz = cmd.base_velocity[2];

                        let total_frames = (cmd.lifespan as f64 * TIME_SCALE).floor() as u32;
                        let mut sctx = ExprContext::new();
                        sctx.set("x", cx);
                        sctx.set("y", cy);
                        sctx.set("z", cz);
                        sctx.set("vx", cur_vx);
                        sctx.set("vy", cur_vy);
                        sctx.set("vz", cur_vz);
                        sctx.set("cr", ctx.get("cr"));
                        sctx.set("cg", ctx.get("cg"));
                        sctx.set("cb", ctx.get("cb"));
                        sctx.set("alpha", ctx.get("alpha"));
                        sctx.set("mpsize", 0.1);
                        sctx.set("age", 0.0);
                        sctx.set("t", 0.0);
                        sctx.set("destory", 0.0);

                        for f in 0..total_frames {
                            sctx.set("age", f as f64 / TIME_SCALE);
                            sctx.set("t", f as f64 / TIME_SCALE);
                            sctx.set("x", cur_x - cmd.center[0]);
                            sctx.set("y", cur_y - cmd.center[1]);
                            sctx.set("z", cur_z - cmd.center[2]);
                            sctx.set("vx", cur_vx);
                            sctx.set("vy", cur_vy);
                            sctx.set("vz", cur_vz);

                            if let Some(ref stmts) = speed_stmts {
                                exec_stmts(stmts, &mut sctx);
                                cur_vx = sctx.get("vx");
                                cur_vy = sctx.get("vy");
                                cur_vz = sctx.get("vz");
                                cur_x = cmd.center[0] + sctx.get("x");
                                cur_y = cmd.center[1] + sctx.get("y");
                                cur_z = cmd.center[2] + sctx.get("z");
                            }

                            if sctx.get("destory") != 0.0 {
                                break;
                            }

                            let cr_val = sctx.get("cr");
                            let cg_val = sctx.get("cg");
                            let cb_val = sctx.get("cb");
                            let ca_val = sctx.get("alpha");

                            track.keyframes.push(Keyframe {
                                tick: f,
                                x: cur_x,
                                y: cur_y,
                                z: cur_z,
                                r: (cr_val * 255.0).clamp(0.0, 255.0) as u8,
                                g: (cg_val * 255.0).clamp(0.0, 255.0) as u8,
                                b: (cb_val * 255.0).clamp(0.0, 255.0) as u8,
                                a: (ca_val * 255.0).clamp(0.0, 255.0) as u8,
                                size: sctx.get("mpsize"),
                            });

                            cur_x += cur_vx / TIME_SCALE;
                            cur_y += cur_vy / TIME_SCALE;
                            cur_z += cur_vz / TIME_SCALE;
                        }

                        if !track.keyframes.is_empty() {
                            tracks.push(track);
                        }
                    }
                    cz += step;
                }
                cy += step;
            }
            cx += step;
        }
        return (tracks, current_id);
    }

    if cmd.config.is_normal {
        let speed_stmts = cmd.speed_expr.as_deref().and_then(compile_expr);

        for _ in 0..cmd.count {
            let mut track = Track {
                id: current_id,
                keyframes: Vec::new(),
            };
            current_id += 1;

            let u: f64 = rng.gen();
            let v: f64 = rng.gen();
            let theta = 2.0 * PI * u;
            let phi = (2.0_f64 * v - 1.0).acos();
            let r = rng.gen::<f64>().cbrt();

            let unit_x = r * phi.sin() * theta.cos();
            let unit_y = r * phi.sin() * theta.sin();
            let unit_z = r * phi.cos();

            let offset_x = unit_x * (cmd.range[0] / 2.0);
            let offset_y = unit_y * (cmd.range[1] / 2.0);
            let offset_z = unit_z * (cmd.range[2] / 2.0);

            let mut cur_x = cmd.center[0] + offset_x;
            let mut cur_y = cmd.center[1] + offset_y;
            let mut cur_z = cmd.center[2] + offset_z;

            let mut cur_vx = cmd.base_velocity[0];
            let mut cur_vy = cmd.base_velocity[1];
            let mut cur_vz = cmd.base_velocity[2];

            let mut ctx = ExprContext::new();
            ctx.set("x", offset_x);
            ctx.set("y", offset_y);
            ctx.set("z", offset_z);
            ctx.set("vx", cur_vx);
            ctx.set("vy", cur_vy);
            ctx.set("vz", cur_vz);
            ctx.set("cr", cmd.color[0]);
            ctx.set("cg", cmd.color[1]);
            ctx.set("cb", cmd.color[2]);
            ctx.set("alpha", cmd.color[3]);
            ctx.set("s1", 0.0);
            ctx.set("s2", 0.0);
            ctx.set("dis", 0.0);
            ctx.set("mpsize", 0.1);
            ctx.set("age", 0.0);
            ctx.set("t", 0.0);
            ctx.set("destory", 0.0);

            let total_frames = (cmd.lifespan as f64 * TIME_SCALE).floor() as u32;

            for f in 0..total_frames {
                ctx.set("age", f as f64 / TIME_SCALE);
                ctx.set("t", f as f64 / TIME_SCALE);
                ctx.set("x", cur_x - cmd.center[0]);
                ctx.set("y", cur_y - cmd.center[1]);
                ctx.set("z", cur_z - cmd.center[2]);
                ctx.set("vx", cur_vx);
                ctx.set("vy", cur_vy);
                ctx.set("vz", cur_vz);

                if let Some(ref stmts) = speed_stmts {
                    exec_stmts(stmts, &mut ctx);
                    cur_vx = ctx.get("vx");
                    cur_vy = ctx.get("vy");
                    cur_vz = ctx.get("vz");
                    cur_x = cmd.center[0] + ctx.get("x");
                    cur_y = cmd.center[1] + ctx.get("y");
                    cur_z = cmd.center[2] + ctx.get("z");
                }

                if ctx.get("destory") == 1.0 {
                    break;
                }

                let cr_val = ctx.get("cr");
                let cg_val = ctx.get("cg");
                let cb_val = ctx.get("cb");
                let ca_val = ctx.get("alpha");

                track.keyframes.push(Keyframe {
                    tick: f,
                    x: cur_x,
                    y: cur_y,
                    z: cur_z,
                    r: (cr_val * 255.0).clamp(0.0, 255.0) as u8,
                    g: (cg_val * 255.0).clamp(0.0, 255.0) as u8,
                    b: (cb_val * 255.0).clamp(0.0, 255.0) as u8,
                    a: (ca_val * 255.0).clamp(0.0, 255.0) as u8,
                    size: ctx.get("mpsize"),
                });

                cur_x += cur_vx / TIME_SCALE;
                cur_y += cur_vy / TIME_SCALE;
                cur_z += cur_vz / TIME_SCALE;
            }

            if !track.keyframes.is_empty() {
                tracks.push(track);
            }
        }

        return (tracks, current_id);
    }

    // ─── Parameter modes ───

    let shape_stmts = cmd.shape_expr.as_deref().and_then(compile_expr);
    let speed_stmts = cmd.speed_expr.as_deref().and_then(compile_expr);
    let safe_step = cmd.t_step.abs();
    let max_particles = 100_000u32;
    let epsilon = 0.000001;
    let mut loop_count = 0u32;
    let mut particle_index = 0u32;

    let mut t_param = cmd.t_begin;
    while t_param < cmd.t_end + epsilon {
        loop_count += 1;
        if loop_count > max_particles {
            break;
        }

        let mut track = Track {
            id: current_id,
            keyframes: Vec::new(),
        };
        current_id += 1;

        let mut ctx = ExprContext::new();
        ctx.set("t", t_param);
        ctx.set("x", 0.0);
        ctx.set("y", 0.0);
        ctx.set("z", 0.0);
        ctx.set("vx", cmd.base_velocity[0]);
        ctx.set("vy", cmd.base_velocity[1]);
        ctx.set("vz", cmd.base_velocity[2]);
        ctx.set("cr", cmd.color[0]);
        ctx.set("cg", cmd.color[1]);
        ctx.set("cb", cmd.color[2]);
        ctx.set("alpha", cmd.color[3]);
        ctx.set("s1", 0.0);
        ctx.set("s2", 0.0);
        ctx.set("dis", 0.0);
        ctx.set("mpsize", 0.1);
        ctx.set("age", 0.0);
        ctx.set("destory", 0.0);

        if let Some(ref stmts) = shape_stmts {
            exec_stmts(stmts, &mut ctx);
        }

        if cmd.config.is_polar {
            let dis = ctx.get("dis");
            let s1 = ctx.get("s1");
            let s2 = ctx.get("s2");
            ctx.set("x", dis * s2.cos() * s1.cos());
            ctx.set("y", dis * s2.sin());
            ctx.set("z", dis * s2.cos() * s1.sin());
        }

        let mut cur_x = cmd.center[0] + ctx.get("x");
        let mut cur_y = cmd.center[1] + ctx.get("y");
        let mut cur_z = cmd.center[2] + ctx.get("z");
        let mut cur_vx = ctx.get("vx");
        let mut cur_vy = ctx.get("vy");
        let mut cur_vz = ctx.get("vz");

        let start_tick_offset = if cmd.config.is_animated {
            (particle_index / cmd.cpt.max(1)) as u32
        } else {
            0
        };
        particle_index += 1;

        let total_frames = (cmd.lifespan as f64 * TIME_SCALE).floor() as u32;

        for f in 0..total_frames {
            ctx.set("age", f as f64 / TIME_SCALE);
            ctx.set("t", f as f64 / TIME_SCALE);
            ctx.set("x", cur_x - cmd.center[0]);
            ctx.set("y", cur_y - cmd.center[1]);
            ctx.set("z", cur_z - cmd.center[2]);
            ctx.set("vx", cur_vx);
            ctx.set("vy", cur_vy);
            ctx.set("vz", cur_vz);

            if let Some(ref stmts) = speed_stmts {
                exec_stmts(stmts, &mut ctx);
                cur_vx = ctx.get("vx");
                cur_vy = ctx.get("vy");
                cur_vz = ctx.get("vz");
                cur_x = cmd.center[0] + ctx.get("x");
                cur_y = cmd.center[1] + ctx.get("y");
                cur_z = cmd.center[2] + ctx.get("z");
            }

            if ctx.get("destory") == 1.0 {
                break;
            }

            let cr_val = ctx.get("cr");
            let cg_val = ctx.get("cg");
            let cb_val = ctx.get("cb");
            let ca_val = ctx.get("alpha");

            track.keyframes.push(Keyframe {
                tick: start_tick_offset + f,
                x: cur_x,
                y: cur_y,
                z: cur_z,
                r: (cr_val * 255.0).clamp(0.0, 255.0) as u8,
                g: (cg_val * 255.0).clamp(0.0, 255.0) as u8,
                b: (cb_val * 255.0).clamp(0.0, 255.0) as u8,
                a: (ca_val * 255.0).clamp(0.0, 255.0) as u8,
                size: ctx.get("mpsize"),
            });

            cur_x += cur_vx / TIME_SCALE;
            cur_y += cur_vy / TIME_SCALE;
            cur_z += cur_vz / TIME_SCALE;
        }

        if !track.keyframes.is_empty() {
            tracks.push(track);
        }

        t_param += safe_step;
    }

    (tracks, current_id)
}

// ─────────────────────── Tracks → Frame Snapshots ───────────────────────

fn tracks_to_frames(tracks: &[Track]) -> Vec<Vec<Particle>> {
    if tracks.is_empty() {
        return vec![];
    }

    let mut max_tick: u32 = 0;
    for t in tracks {
        if let Some(last) = t.keyframes.last() {
            if last.tick > max_tick {
                max_tick = last.tick;
            }
        }
    }

    let total = (max_tick + 1) as usize;
    let mut frames: Vec<Vec<Particle>> = Vec::with_capacity(total);
    for _ in 0..total {
        frames.push(Vec::new());
    }

    for t in tracks {
        for k in &t.keyframes {
            let fi = k.tick as usize;
            if fi < frames.len() {
                frames[fi].push(Particle {
                    id: t.id,
                    pos: [k.x as f32, k.y as f32, k.z as f32],
                    color: [k.r, k.g, k.b, k.a],
                    size: k.size as f32,
                    tex_id: 0,
                    seq_index: 0,
                });
            }
        }
    }

    frames
}

// ─────────────────────── Public API ───────────────────────

/// Compile particleex commands text into frame snapshots.
/// Returns (frames, target_fps).
pub fn compile(commands_text: &str) -> Result<(Vec<Vec<Particle>>, u16), String> {
    let lines: Vec<&str> = commands_text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    let mut all_tracks: Vec<Track> = Vec::new();
    let mut p_id: i32 = 0;
    let mut errors = Vec::new();

    for line in &lines {
        let lower = line.to_lowercase();
        if !lower.starts_with("particleex")
            && !lower.starts_with("/particleex")
            && !lower.starts_with("particlex")
            && !lower.starts_with("/particlex")
        {
            continue;
        }
        match parse_command(line) {
            Some(cmd) => {
                let (tracks, next_id) = generate_tracks(&cmd, p_id);
                p_id = next_id;
                all_tracks.extend(tracks);
            }
            None => {
                errors.push(format!("Failed to parse: {}", line));
            }
        }
    }

    if all_tracks.is_empty() {
        return Err(if errors.is_empty() {
            "No particles generated from commands".into()
        } else {
            errors.join("\n")
        });
    }

    let frames = tracks_to_frames(&all_tracks);
    // Default 60 FPS matching the JS implementation
    Ok((frames, 60))
}
