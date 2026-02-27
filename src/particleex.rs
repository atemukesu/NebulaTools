/// Particleex command compiler.
///
/// Ports the JavaScript Particleex → NBL pipeline to Rust:
///   1. Pest PEG parser + AST builder + tree-walking evaluator
///   2. Particleex command parser (normal, parameter, polarparameter, tick*, rgba*)
///   3. Track generator  →  Vec<Vec<Particle>>  (frame snapshots)
use crate::player::Particle;
use pest::iterators::Pair;
use pest::Parser as PestParser;
use pest_derive::Parser;
use rand::Rng;
use std::collections::HashMap;
use std::f64::consts::{E, PI};

// ─────────────────────── Constants ───────────────────────

const TIME_SCALE: f64 = 3.0;

// ─────────────────────── Pest Grammar ───────────────────────

#[derive(Parser)]
#[grammar = "particleex.pest"]
struct ExprParser;

// ─── AST ───

#[derive(Debug, Clone)]
pub enum Expr {
    Num(f64),
    Var(String),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryNeg(Box<Expr>),
    UnaryNot(Box<Expr>),
    Call(String, Vec<Expr>),
    #[allow(dead_code)]
    Conditional(Box<Expr>, Box<Expr>, Box<Expr>),
    MatrixBuilder(Vec<Vec<Expr>>),
    Assign(String, Box<Expr>),
    MultiAssign(Vec<String>, Vec<Expr>),
}

#[derive(Debug, Clone, Copy)]
pub enum BinOp {
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

// ─── Statement-level types ───

#[derive(Debug, Clone)]
pub enum Stmt {
    ExprStmt(Expr),
}

// ─── Pest pair → AST conversion ───

fn build_expr(pair: Pair<Rule>) -> Expr {
    match pair.as_rule() {
        Rule::expr | Rule::stmt => build_expr(pair.into_inner().next().unwrap()),
        Rule::number => {
            let n: f64 = pair.as_str().parse().unwrap_or(0.0);
            Expr::Num(n)
        }
        Rule::var => Expr::Var(pair.as_str().to_string()),
        Rule::call => {
            let mut inner = pair.into_inner();
            let name = inner.next().unwrap().as_str().to_string();
            let args: Vec<Expr> = inner
                .filter(|p| p.as_rule() == Rule::arg_list)
                .flat_map(|p| p.into_inner())
                .map(build_expr)
                .collect();
            Expr::Call(name, args)
        }
        Rule::paren_expr => {
            let mut inner = pair.into_inner();
            let first = build_expr(inner.next().unwrap());
            let continuations: Vec<Pair<Rule>> =
                inner.filter(|p| p.as_rule() == Rule::row_cont).collect();
            if continuations.is_empty() {
                return first;
            }
            // Build matrix rows: ,, starts new row, , continues current row
            let mut rows: Vec<Vec<Expr>> = Vec::new();
            let mut current_row = vec![first];
            for cont in continuations {
                let mut ci = cont.into_inner();
                let sep = ci.next().unwrap();
                let val = build_expr(ci.next().unwrap());
                if sep.as_str() == ",," {
                    rows.push(std::mem::take(&mut current_row));
                    current_row.push(val);
                } else {
                    current_row.push(val);
                }
            }
            rows.push(current_row);
            Expr::MatrixBuilder(rows)
        }
        Rule::assign_expr => {
            let mut inner = pair.into_inner();
            let lhs_pair = inner.next().unwrap(); // lhs_list
            let rhs_pair = inner.next().unwrap(); // rhs_list
            let names: Vec<String> = lhs_pair
                .into_inner()
                .filter(|p| p.as_rule() == Rule::ident)
                .map(|p| p.as_str().to_string())
                .collect();
            let exprs: Vec<Expr> = rhs_pair.into_inner().map(build_expr).collect();
            if names.len() == 1 && exprs.len() == 1 {
                Expr::Assign(
                    names[0].clone(),
                    Box::new(exprs.into_iter().next().unwrap()),
                )
            } else {
                Expr::MultiAssign(names, exprs)
            }
        }
        Rule::neg => {
            let inner = pair.into_inner().next().unwrap();
            Expr::UnaryNeg(Box::new(build_expr(inner)))
        }
        Rule::not => {
            let inner = pair.into_inner().next().unwrap();
            Expr::UnaryNot(Box::new(build_expr(inner)))
        }
        Rule::or_expr | Rule::and_expr | Rule::cmp_expr | Rule::add_expr | Rule::mul_expr => {
            let mut inner = pair.into_inner();
            let mut left = build_expr(inner.next().unwrap());
            while let Some(op_pair) = inner.next() {
                let op = match op_pair.as_str() {
                    "||" | "|" => BinOp::Or,
                    "&&" | "&" => BinOp::And,
                    "==" => BinOp::Eq,
                    "!=" => BinOp::Ne,
                    "<=" => BinOp::Le,
                    ">=" => BinOp::Ge,
                    "<" => BinOp::Lt,
                    ">" => BinOp::Gt,
                    "+" => BinOp::Add,
                    "-" => BinOp::Sub,
                    "*" => BinOp::Mul,
                    "/" => BinOp::Div,
                    "%" => BinOp::Mod,
                    _ => BinOp::Add,
                };
                let right = build_expr(inner.next().unwrap());
                left = Expr::BinOp(Box::new(left), op, Box::new(right));
            }
            left
        }
        Rule::pow_expr => {
            let mut inner = pair.into_inner();
            let base = build_expr(inner.next().unwrap());
            if let Some(exp) = inner.next() {
                Expr::BinOp(Box::new(base), BinOp::Pow, Box::new(build_expr(exp)))
            } else {
                base
            }
        }
        Rule::unary | Rule::primary => build_expr(pair.into_inner().next().unwrap()),
        _ => Expr::Num(0.0),
    }
}

fn build_stmt(pair: Pair<Rule>) -> Stmt {
    match pair.as_rule() {
        Rule::stmt => Stmt::ExprStmt(build_expr(pair.into_inner().next().unwrap())),
        _ => Stmt::ExprStmt(Expr::Num(0.0)),
    }
}

pub fn parse_statements_pest(src: &str) -> Vec<Stmt> {
    let parsed = ExprParser::parse(Rule::program, src);
    match parsed {
        Ok(mut pairs) => {
            let program = pairs.next().unwrap();
            let mut stmts = Vec::new();
            for pair in program.into_inner() {
                if pair.as_rule() == Rule::stmt {
                    stmts.push(build_stmt(pair));
                }
            }
            stmts
        }
        Err(e) => {
            println!("Parse Error: {:?}", e);
            Vec::new()
        }
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    Num(f64),
    Matrix(Vec<Vec<f64>>),
}

impl Value {
    pub fn as_num(&self) -> f64 {
        match self {
            Value::Num(n) => *n,
            Value::Matrix(m) => m
                .first()
                .and_then(|row| row.first())
                .copied()
                .unwrap_or(0.0),
        }
    }

    pub fn is_true(&self) -> bool {
        match self {
            Value::Num(n) => *n != 0.0,
            Value::Matrix(m) => !m.is_empty() && !m[0].is_empty() && m[0][0] != 0.0,
        }
    }
}

// ─── Evaluator context ───

#[derive(Clone)]
pub struct ExprContext {
    pub vars: HashMap<String, Value>,
}

impl ExprContext {
    pub fn new() -> Self {
        let mut vars = HashMap::new();
        vars.insert("PI".into(), Value::Num(PI));
        vars.insert("E".into(), Value::Num(E));
        Self { vars }
    }

    pub fn get(&self, name: &str) -> Value {
        self.vars.get(name).cloned().unwrap_or(Value::Num(0.0))
    }

    pub fn set(&mut self, name: &str, val: Value) {
        self.vars.insert(name.to_string(), val);
    }
}

pub fn eval_expr(expr: &Expr, ctx: &mut ExprContext) -> Value {
    match expr {
        Expr::Num(n) => Value::Num(*n),
        Expr::Var(name) => ctx.get(name),
        Expr::BinOp(l, op, r) => {
            // 修复 3：将 And 和 Or 提出来做短路判断！
            match op {
                BinOp::And => {
                    let lv = eval_expr(l, ctx);
                    if !lv.is_true() {
                        return Value::Num(0.0);
                    }
                    let rv = eval_expr(r, ctx);
                    return Value::Num(if rv.is_true() { 1.0 } else { 0.0 });
                }
                BinOp::Or => {
                    let lv = eval_expr(l, ctx);
                    if lv.is_true() {
                        return Value::Num(1.0);
                    }
                    let rv = eval_expr(r, ctx);
                    return Value::Num(if rv.is_true() { 1.0 } else { 0.0 });
                }
                _ => {}
            }

            // 其他运算符正常求值两边
            let lv = eval_expr(l, ctx);
            let rv = eval_expr(r, ctx);
            match op {
                BinOp::Add => match (lv, rv) {
                    (Value::Num(a), Value::Num(b)) => Value::Num(a + b),
                    (Value::Matrix(mut m1), Value::Matrix(m2)) => {
                        if !m1.is_empty()
                            && !m2.is_empty()
                            && m1.len() == m2.len()
                            && m1[0].len() == m2[0].len()
                        {
                            for i in 0..m1.len() {
                                for j in 0..m1[0].len() {
                                    m1[i][j] += m2[i][j];
                                }
                            }
                            Value::Matrix(m1)
                        } else {
                            Value::Num(0.0)
                        }
                    }
                    _ => Value::Num(0.0),
                },
                BinOp::Sub => match (lv, rv) {
                    (Value::Num(a), Value::Num(b)) => Value::Num(a - b),
                    (Value::Matrix(mut m1), Value::Matrix(m2)) => {
                        if !m1.is_empty()
                            && !m2.is_empty()
                            && m1.len() == m2.len()
                            && m1[0].len() == m2[0].len()
                        {
                            for i in 0..m1.len() {
                                for j in 0..m1[0].len() {
                                    m1[i][j] -= m2[i][j];
                                }
                            }
                            Value::Matrix(m1)
                        } else {
                            Value::Num(0.0)
                        }
                    }
                    _ => Value::Num(0.0),
                },
                BinOp::Mul => match (lv, rv) {
                    (Value::Num(a), Value::Num(b)) => Value::Num(a * b),
                    (Value::Num(s), Value::Matrix(mut m))
                    | (Value::Matrix(mut m), Value::Num(s)) => {
                        for row in &mut m {
                            for v in row {
                                *v *= s;
                            }
                        }
                        Value::Matrix(m)
                    }
                    (Value::Matrix(m1), Value::Matrix(m2)) => {
                        if m1.is_empty() || m2.is_empty() || m1[0].len() != m2.len() {
                            Value::Num(0.0)
                        } else {
                            let r1 = m1.len();
                            let c1 = m1[0].len();
                            let c2 = m2[0].len();
                            let mut result = vec![vec![0.0; c2]; r1];
                            for i in 0..r1 {
                                for j in 0..c2 {
                                    for k in 0..c1 {
                                        result[i][j] += m1[i][k] * m2[k][j];
                                    }
                                }
                            }
                            Value::Matrix(result)
                        }
                    }
                },
                BinOp::Div => {
                    let (la, ra) = (lv.as_num(), rv.as_num());
                    if ra.abs() < 1e-15 {
                        Value::Num(0.0)
                    } else {
                        Value::Num(la / ra)
                    }
                }
                BinOp::Mod => {
                    let (la, ra) = (lv.as_num(), rv.as_num());
                    if ra.abs() < 1e-15 {
                        Value::Num(0.0)
                    } else {
                        Value::Num(la % ra)
                    }
                }
                BinOp::Pow => Value::Num(lv.as_num().powf(rv.as_num())),
                BinOp::Eq => Value::Num(if (lv.as_num() - rv.as_num()).abs() < 1e-10 {
                    1.0
                } else {
                    0.0
                }),
                BinOp::Ne => Value::Num(if (lv.as_num() - rv.as_num()).abs() >= 1e-10 {
                    1.0
                } else {
                    0.0
                }),
                BinOp::Lt => Value::Num(if lv.as_num() < rv.as_num() { 1.0 } else { 0.0 }),
                BinOp::Gt => Value::Num(if lv.as_num() > rv.as_num() { 1.0 } else { 0.0 }),
                BinOp::Le => Value::Num(if lv.as_num() <= rv.as_num() { 1.0 } else { 0.0 }),
                BinOp::Ge => Value::Num(if lv.as_num() >= rv.as_num() { 1.0 } else { 0.0 }),
                BinOp::And => Value::Num(if lv.is_true() && rv.is_true() {
                    1.0
                } else {
                    0.0
                }),
                BinOp::Or => Value::Num(if lv.is_true() || rv.is_true() {
                    1.0
                } else {
                    0.0
                }),
            }
        }
        Expr::UnaryNeg(e) => {
            let v = eval_expr(e, ctx);
            match v {
                Value::Num(n) => Value::Num(-n),
                Value::Matrix(mut m) => {
                    for row in &mut m {
                        for x in row {
                            *x = -*x;
                        }
                    }
                    Value::Matrix(m)
                }
            }
        }
        Expr::UnaryNot(e) => Value::Num(if eval_expr(e, ctx).is_true() {
            0.0
        } else {
            1.0
        }),
        Expr::Call(name, args) => {
            let mut rng = rand::thread_rng();
            let a: Vec<Value> = args.iter().map(|e| eval_expr(e, ctx)).collect();
            let nums: Vec<f64> = a.iter().map(|v| v.as_num()).collect();

            match name.as_str() {
                "sin" => Value::Num(nums.first().copied().unwrap_or(0.0).sin()),
                "cos" => Value::Num(nums.first().copied().unwrap_or(0.0).cos()),
                "tan" => Value::Num(nums.first().copied().unwrap_or(0.0).tan()),
                "asin" => Value::Num(nums.first().copied().unwrap_or(0.0).asin()),
                "acos" => Value::Num(nums.first().copied().unwrap_or(0.0).acos()),
                "atan" => Value::Num(nums.first().copied().unwrap_or(0.0).atan()),
                "atan2" => {
                    let y = nums.first().copied().unwrap_or(0.0);
                    let x = nums.get(1).copied().unwrap_or(0.0);
                    Value::Num(y.atan2(x))
                }
                "sinh" => Value::Num(nums.first().copied().unwrap_or(0.0).sinh()),
                "cosh" => Value::Num(nums.first().copied().unwrap_or(0.0).cosh()),
                "tanh" => Value::Num(nums.first().copied().unwrap_or(0.0).tanh()),
                "exp" => Value::Num(nums.first().copied().unwrap_or(0.0).exp()),
                "log" | "ln" => Value::Num(nums.first().copied().unwrap_or(0.0).ln()),
                "log10" => Value::Num(nums.first().copied().unwrap_or(0.0).log10()),
                "expm1" => Value::Num(nums.first().copied().unwrap_or(0.0).exp_m1()),
                "log1p" => Value::Num(nums.first().copied().unwrap_or(0.0).ln_1p()),
                "pow" => {
                    let base = nums.first().copied().unwrap_or(0.0);
                    let exp = nums.get(1).copied().unwrap_or(0.0);
                    Value::Num(base.powf(exp))
                }
                "sqrt" => Value::Num(nums.first().copied().unwrap_or(0.0).sqrt()),
                "cbrt" => Value::Num(nums.first().copied().unwrap_or(0.0).cbrt()),
                "hypot" => {
                    let x = nums.first().copied().unwrap_or(0.0);
                    let y = nums.get(1).copied().unwrap_or(0.0);
                    Value::Num(x.hypot(y))
                }
                "ceil" => Value::Num(nums.first().copied().unwrap_or(0.0).ceil()),
                "floor" => Value::Num(nums.first().copied().unwrap_or(0.0).floor()),
                "round" | "rint" => Value::Num(nums.first().copied().unwrap_or(0.0).round()),
                "max" => {
                    let x = nums.first().copied().unwrap_or(0.0);
                    let y = nums.get(1).copied().unwrap_or(0.0);
                    Value::Num(x.max(y))
                }
                "min" => {
                    let x = nums.first().copied().unwrap_or(0.0);
                    let y = nums.get(1).copied().unwrap_or(0.0);
                    Value::Num(x.min(y))
                }
                "abs" | "signum" => {
                    let v = nums.first().copied().unwrap_or(0.0);
                    if name == "abs" {
                        Value::Num(v.abs())
                    } else {
                        Value::Num(v.signum())
                    }
                }
                "random" => Value::Num(rng.gen::<f64>()),
                "toRadians" => Value::Num(nums.first().copied().unwrap_or(0.0).to_radians()),
                "toDegrees" => Value::Num(nums.first().copied().unwrap_or(0.0).to_degrees()),
                "clamp" => {
                    let val = nums.first().copied().unwrap_or(0.0);
                    let min = nums.get(1).copied().unwrap_or(0.0);
                    let max = nums.get(2).copied().unwrap_or(0.0);
                    Value::Num(val.clamp(min, max))
                }
                "lerp" => {
                    let delta = nums.first().copied().unwrap_or(0.0);
                    let start = nums.get(1).copied().unwrap_or(0.0);
                    let end = nums.get(2).copied().unwrap_or(0.0);
                    Value::Num(start + delta * (end - start))
                }
                "lerpInt" => {
                    let delta = nums.first().copied().unwrap_or(0.0);
                    let start = nums.get(1).copied().unwrap_or(0.0);
                    let end = nums.get(2).copied().unwrap_or(0.0);
                    Value::Num((start + delta * (end - start)).floor())
                }
                "floorMod" | "IEEEremainder" => {
                    let x = nums.first().copied().unwrap_or(0.0);
                    let y = nums.get(1).copied().unwrap_or(1.0);
                    Value::Num(x.rem_euclid(y))
                }
                "fma" => {
                    let x = nums.first().copied().unwrap_or(0.0);
                    let y = nums.get(1).copied().unwrap_or(0.0);
                    let z = nums.get(2).copied().unwrap_or(0.0);
                    Value::Num(x.mul_add(y, z))
                }
                "copySign" => {
                    let magnitude = nums.first().copied().unwrap_or(0.0);
                    let sign = nums.get(1).copied().unwrap_or(0.0);
                    Value::Num(magnitude.copysign(sign))
                }
                "getExponent" => {
                    let v = nums.first().copied().unwrap_or(0.0);
                    Value::Num(((v.to_bits() >> 52) & 0x7FF) as f64 - 1023.0)
                }
                "addExact" => Value::Num(
                    nums.first().copied().unwrap_or(0.0) + nums.get(1).copied().unwrap_or(0.0),
                ),
                "multiplyExact" => Value::Num(
                    nums.first().copied().unwrap_or(0.0) * nums.get(1).copied().unwrap_or(0.0),
                ),
                "nextUp" => {
                    let v = nums.first().copied().unwrap_or(0.0);
                    Value::Num(f64::from_bits(v.to_bits().wrapping_add(1)))
                }
                "nextDown" => {
                    let v = nums.first().copied().unwrap_or(0.0);
                    Value::Num(f64::from_bits(v.to_bits().wrapping_sub(1)))
                }
                "scalb" => {
                    let d = nums.first().copied().unwrap_or(0.0);
                    let sf = nums.get(1).copied().unwrap_or(0.0) as i32;
                    Value::Num(d * 2.0_f64.powi(sf))
                }
                "translate" => {
                    let x = nums.get(0).copied().unwrap_or(0.0);
                    let y = nums.get(1).copied().unwrap_or(0.0);
                    let z = nums.get(2).copied().unwrap_or(0.0);
                    Value::Matrix(vec![
                        vec![1.0, 0.0, 0.0, x],
                        vec![0.0, 1.0, 0.0, y],
                        vec![0.0, 0.0, 1.0, z],
                        vec![0.0, 0.0, 0.0, 1.0],
                    ])
                }
                "scale" => {
                    let x = nums.get(0).copied().unwrap_or(1.0);
                    let y = nums.get(1).copied().unwrap_or(1.0);
                    let z = nums.get(2).copied().unwrap_or(1.0);
                    Value::Matrix(vec![
                        vec![x, 0.0, 0.0, 0.0],
                        vec![0.0, y, 0.0, 0.0],
                        vec![0.0, 0.0, z, 0.0],
                        vec![0.0, 0.0, 0.0, 1.0],
                    ])
                }
                "rotate" | "rotateDeg" => {
                    let mut p = nums.get(0).copied().unwrap_or(0.0);
                    let mut y = nums.get(1).copied().unwrap_or(0.0);
                    let mut r = nums.get(2).copied().unwrap_or(0.0);

                    if name == "rotateDeg" {
                        p = p.to_radians();
                        y = y.to_radians();
                        r = r.to_radians();
                    }

                    let (cp, sp) = (p.cos(), p.sin());
                    let (cy, sy) = (y.cos(), y.sin());
                    let (cr, sr) = (r.cos(), r.sin());

                    // 按照 Z * Y * X 顺序复合的欧拉角矩阵
                    Value::Matrix(vec![
                        vec![cy * cr, sy * sp * cr - cp * sr, sy * cp * cr + sp * sr, 0.0],
                        vec![cy * sr, sy * sp * sr + cp * cr, sy * cp * sr - sp * cr, 0.0],
                        vec![-sy, cy * sp, cy * cp, 0.0],
                        vec![0.0, 0.0, 0.0, 1.0],
                    ])
                }
                "transpose" => {
                    if let Some(Value::Matrix(m)) = a.get(0) {
                        if m.is_empty() || m[0].is_empty() {
                            return Value::Matrix(vec![]);
                        }
                        let rows = m.len();
                        let cols = m[0].len();
                        let mut res = vec![vec![0.0; rows]; cols];
                        for i in 0..rows {
                            for j in 0..cols {
                                res[j][i] = m[i][j];
                            }
                        }
                        Value::Matrix(res)
                    } else {
                        Value::Num(0.0)
                    }
                }
                _ => Value::Num(0.0),
            }
        }
        Expr::Conditional(cond, then, else_) => {
            if eval_expr(cond, ctx).is_true() {
                eval_expr(then, ctx)
            } else {
                eval_expr(else_, ctx)
            }
        }
        Expr::MatrixBuilder(rows) => {
            let mut res_rows = Vec::new();
            for r in rows {
                let mut res_row = Vec::new();
                for e in r {
                    res_row.push(eval_expr(e, ctx).as_num());
                }
                res_rows.push(res_row);
            }
            Value::Matrix(res_rows)
        }
        Expr::Assign(name, val_expr) => {
            let val = eval_expr(val_expr, ctx);
            ctx.set(name, val.clone());
            val
        }
        Expr::MultiAssign(names, exprs) => {
            let vals: Vec<Value> = exprs.iter().map(|e| eval_expr(e, ctx)).collect();
            if vals.len() == 1 {
                if let Value::Matrix(m) = &vals[0] {
                    let mut flat = Vec::new();
                    for row in m {
                        for val in row {
                            flat.push(*val);
                        }
                    }
                    for (i, name) in names.iter().enumerate() {
                        let v = flat.get(i).copied().unwrap_or(0.0);
                        ctx.set(name, Value::Num(v));
                    }
                    return vals[0].clone();
                }
            }

            let mut last_v = Value::Num(0.0);
            for (i, name) in names.iter().enumerate() {
                let v = vals.get(i).cloned().unwrap_or(Value::Num(0.0));
                ctx.set(name, v.clone());
                last_v = v;
            }
            last_v
        }
    }
}

pub fn exec_stmts(stmts: &[Stmt], ctx: &mut ExprContext) -> Value {
    let mut last_val = Value::Num(0.0);
    for stmt in stmts {
        last_val = match stmt {
            Stmt::ExprStmt(expr) => eval_expr(expr, ctx),
        };
    }
    last_val
}

/// Compile an expression string into executable statements. Returns None if empty.
pub fn compile_expr(src: &str) -> Option<Vec<Stmt>> {
    let src = src.trim();
    if src.is_empty() || src == "null" {
        return None;
    }
    let stmts = parse_statements_pest(src);
    if stmts.is_empty() {
        return None;
    }
    Some(stmts)
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
    let num = if s.is_empty() {
        0.0
    } else {
        s.parse::<f64>().unwrap_or(0.0)
    };
    num
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
                    ctx.set("x", Value::Num(cx));
                    ctx.set("y", Value::Num(cy));
                    ctx.set("z", Value::Num(cz));
                    ctx.set("s1", Value::Num(0.0));
                    ctx.set("s2", Value::Num(0.0));
                    ctx.set("dis", Value::Num((cx * cx + cy * cy + cz * cz).sqrt()));
                    ctx.set("cr", Value::Num(cmd.color[0]));
                    ctx.set("cg", Value::Num(cmd.color[1]));
                    ctx.set("cb", Value::Num(cmd.color[2]));
                    ctx.set("alpha", Value::Num(cmd.color[3]));

                    // Evaluate condition: result ≠ 0 means spawn
                    let spawn = if let Some(ref stmts) = cond_stmts {
                        exec_stmts(stmts, &mut ctx).is_true()
                    } else {
                        true
                    };

                    if spawn && !ctx.get("destroy").is_true() {
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
                        sctx.set("x", Value::Num(cx));
                        sctx.set("y", Value::Num(cy));
                        sctx.set("z", Value::Num(cz));
                        sctx.set("vx", Value::Num(cur_vx));
                        sctx.set("vy", Value::Num(cur_vy));
                        sctx.set("vz", Value::Num(cur_vz));
                        sctx.set("cr", ctx.get("cr"));
                        sctx.set("cg", ctx.get("cg"));
                        sctx.set("cb", ctx.get("cb"));
                        sctx.set("alpha", ctx.get("alpha"));
                        sctx.set("mpsize", Value::Num(0.1));
                        sctx.set("age", Value::Num(0.0));
                        sctx.set("t", Value::Num(0.0));
                        sctx.set("destroy", Value::Num(0.0));

                        for f in 0..total_frames {
                            sctx.set("age", Value::Num(f as f64 / TIME_SCALE));
                            sctx.set("t", Value::Num(f as f64 / TIME_SCALE));
                            sctx.set("x", Value::Num(cur_x - cmd.center[0]));
                            sctx.set("y", Value::Num(cur_y - cmd.center[1]));
                            sctx.set("z", Value::Num(cur_z - cmd.center[2]));
                            sctx.set("vx", Value::Num(cur_vx));
                            sctx.set("vy", Value::Num(cur_vy));
                            sctx.set("vz", Value::Num(cur_vz));

                            if let Some(ref stmts) = speed_stmts {
                                exec_stmts(stmts, &mut sctx);
                                cur_vx = sctx.get("vx").as_num();
                                cur_vy = sctx.get("vy").as_num();
                                cur_vz = sctx.get("vz").as_num();
                                cur_x = cmd.center[0] + sctx.get("x").as_num();
                                cur_y = cmd.center[1] + sctx.get("y").as_num();
                                cur_z = cmd.center[2] + sctx.get("z").as_num();
                            }

                            if sctx.get("destroy").is_true() {
                                break;
                            }

                            let cr_val = sctx.get("cr").as_num();
                            let cg_val = sctx.get("cg").as_num();
                            let cb_val = sctx.get("cb").as_num();
                            let ca_val = sctx.get("alpha").as_num();

                            track.keyframes.push(Keyframe {
                                tick: f,
                                x: cur_x,
                                y: cur_y,
                                z: cur_z,
                                r: (cr_val * 255.0).clamp(0.0, 255.0) as u8,
                                g: (cg_val * 255.0).clamp(0.0, 255.0) as u8,
                                b: (cb_val * 255.0).clamp(0.0, 255.0) as u8,
                                a: (ca_val * 255.0).clamp(0.0, 255.0) as u8,
                                size: sctx.get("mpsize").as_num(),
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
            ctx.set("x", Value::Num(offset_x));
            ctx.set("y", Value::Num(offset_y));
            ctx.set("z", Value::Num(offset_z));
            ctx.set("vx", Value::Num(cur_vx));
            ctx.set("vy", Value::Num(cur_vy));
            ctx.set("vz", Value::Num(cur_vz));
            ctx.set("cr", Value::Num(cmd.color[0]));
            ctx.set("cg", Value::Num(cmd.color[1]));
            ctx.set("cb", Value::Num(cmd.color[2]));
            ctx.set("alpha", Value::Num(cmd.color[3]));
            ctx.set("s1", Value::Num(0.0));
            ctx.set("s2", Value::Num(0.0));
            ctx.set("dis", Value::Num(0.0));
            ctx.set("mpsize", Value::Num(0.1));
            ctx.set("age", Value::Num(0.0));
            ctx.set("t", Value::Num(0.0));
            ctx.set("destroy", Value::Num(0.0));

            let total_frames = (cmd.lifespan as f64 * TIME_SCALE).floor() as u32;

            for f in 0..total_frames {
                ctx.set("age", Value::Num(f as f64 / TIME_SCALE));
                ctx.set("t", Value::Num(f as f64 / TIME_SCALE));
                ctx.set("x", Value::Num(cur_x - cmd.center[0]));
                ctx.set("y", Value::Num(cur_y - cmd.center[1]));
                ctx.set("z", Value::Num(cur_z - cmd.center[2]));
                ctx.set("vx", Value::Num(cur_vx));
                ctx.set("vy", Value::Num(cur_vy));
                ctx.set("vz", Value::Num(cur_vz));

                if let Some(ref stmts) = speed_stmts {
                    exec_stmts(stmts, &mut ctx);
                    cur_vx = ctx.get("vx").as_num();
                    cur_vy = ctx.get("vy").as_num();
                    cur_vz = ctx.get("vz").as_num();
                    cur_x = cmd.center[0] + ctx.get("x").as_num();
                    cur_y = cmd.center[1] + ctx.get("y").as_num();
                    cur_z = cmd.center[2] + ctx.get("z").as_num();
                }

                if ctx.get("destroy").is_true() {
                    break;
                }

                let cr_val = ctx.get("cr").as_num();
                let cg_val = ctx.get("cg").as_num();
                let cb_val = ctx.get("cb").as_num();
                let ca_val = ctx.get("alpha").as_num();

                track.keyframes.push(Keyframe {
                    tick: f,
                    x: cur_x,
                    y: cur_y,
                    z: cur_z,
                    r: (cr_val * 255.0).clamp(0.0, 255.0) as u8,
                    g: (cg_val * 255.0).clamp(0.0, 255.0) as u8,
                    b: (cb_val * 255.0).clamp(0.0, 255.0) as u8,
                    a: (ca_val * 255.0).clamp(0.0, 255.0) as u8,
                    size: ctx.get("mpsize").as_num(),
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
        ctx.set("t", Value::Num(t_param));
        ctx.set("x", Value::Num(0.0));
        ctx.set("y", Value::Num(0.0));
        ctx.set("z", Value::Num(0.0));
        ctx.set("vx", Value::Num(cmd.base_velocity[0]));
        ctx.set("vy", Value::Num(cmd.base_velocity[1]));
        ctx.set("vz", Value::Num(cmd.base_velocity[2]));
        ctx.set("cr", Value::Num(cmd.color[0]));
        ctx.set("cg", Value::Num(cmd.color[1]));
        ctx.set("cb", Value::Num(cmd.color[2]));
        ctx.set("alpha", Value::Num(cmd.color[3]));
        ctx.set("s1", Value::Num(0.0));
        ctx.set("s2", Value::Num(0.0));
        ctx.set("dis", Value::Num(0.0));
        ctx.set("mpsize", Value::Num(0.1));
        ctx.set("age", Value::Num(0.0));
        ctx.set("destroy", Value::Num(0.0));

        if let Some(ref stmts) = shape_stmts {
            exec_stmts(stmts, &mut ctx);
        }

        if cmd.config.is_polar {
            let dis = ctx.get("dis").as_num();
            let s1 = ctx.get("s1").as_num();
            let s2 = ctx.get("s2").as_num();
            ctx.set("x", Value::Num(dis * s2.cos() * s1.cos()));
            ctx.set("y", Value::Num(dis * s2.sin()));
            ctx.set("z", Value::Num(dis * s2.cos() * s1.sin()));
        }

        let mut cur_x = cmd.center[0] + ctx.get("x").as_num();
        let mut cur_y = cmd.center[1] + ctx.get("y").as_num();
        let mut cur_z = cmd.center[2] + ctx.get("z").as_num();
        let mut cur_vx = ctx.get("vx").as_num();
        let mut cur_vy = ctx.get("vy").as_num();
        let mut cur_vz = ctx.get("vz").as_num();

        let start_tick_offset = if cmd.config.is_animated {
            (particle_index / cmd.cpt.max(1)) as u32
        } else {
            0
        };
        particle_index += 1;

        let total_frames = (cmd.lifespan as f64 * TIME_SCALE).floor() as u32;

        for f in 0..total_frames {
            ctx.set("age", Value::Num(f as f64 / TIME_SCALE));
            ctx.set("t", Value::Num(f as f64 / TIME_SCALE));
            ctx.set("x", Value::Num(cur_x - cmd.center[0]));
            ctx.set("y", Value::Num(cur_y - cmd.center[1]));
            ctx.set("z", Value::Num(cur_z - cmd.center[2]));
            ctx.set("vx", Value::Num(cur_vx));
            ctx.set("vy", Value::Num(cur_vy));
            ctx.set("vz", Value::Num(cur_vz));

            if let Some(ref stmts) = speed_stmts {
                exec_stmts(stmts, &mut ctx);
                cur_vx = ctx.get("vx").as_num();
                cur_vy = ctx.get("vy").as_num();
                cur_vz = ctx.get("vz").as_num();
                cur_x = cmd.center[0] + ctx.get("x").as_num();
                cur_y = cmd.center[1] + ctx.get("y").as_num();
                cur_z = cmd.center[2] + ctx.get("z").as_num();
            }

            if ctx.get("destroy").is_true() {
                break;
            }

            let cr_val = ctx.get("cr").as_num();
            let cg_val = ctx.get("cg").as_num();
            let cb_val = ctx.get("cb").as_num();
            let ca_val = ctx.get("alpha").as_num();

            track.keyframes.push(Keyframe {
                tick: start_tick_offset + f,
                x: cur_x,
                y: cur_y,
                z: cur_z,
                r: (cr_val * 255.0).clamp(0.0, 255.0) as u8,
                g: (cg_val * 255.0).clamp(0.0, 255.0) as u8,
                b: (cb_val * 255.0).clamp(0.0, 255.0) as u8,
                a: (ca_val * 255.0).clamp(0.0, 255.0) as u8,
                size: ctx.get("mpsize").as_num(),
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
#[allow(dead_code)]
pub fn compile(commands_text: &str) -> Result<(Vec<Vec<Particle>>, u16), String> {
    let entries = vec![CompileEntry {
        command: commands_text.to_string(),
        start_tick: 0.0,
        position: [0.0; 3],
        duration_override: 0.0,
    }];
    compile_entries(&entries)
}

/// A single compilable entry with optional overrides.
pub struct CompileEntry {
    pub command: String,
    pub start_tick: f64,
    pub position: [f64; 3],
    pub duration_override: f64, // 0 = use command's own value
}

/// Compile multiple entries into merged frame snapshots.
/// Each entry can have its own start time, position, and duration override.
/// Returns (frames, target_fps).
pub fn compile_entries(entries: &[CompileEntry]) -> Result<(Vec<Vec<Particle>>, u16), String> {
    let mut all_tracks: Vec<Track> = Vec::new();
    let mut p_id: i32 = 0;
    let mut errors = Vec::new();

    for entry in entries {
        let lines: Vec<&str> = entry
            .command
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

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
                Some(mut cmd) => {
                    // Apply position override
                    if entry.position != [0.0; 3] {
                        cmd.center = entry.position;
                    }
                    // Apply duration override
                    if entry.duration_override > 0.0 {
                        cmd.lifespan = entry.duration_override as u32;
                    }

                    let (mut tracks, next_id) = generate_tracks(&cmd, p_id);

                    // Apply start tick offset
                    let offset = (entry.start_tick * TIME_SCALE).floor() as u32;
                    if offset > 0 {
                        for track in &mut tracks {
                            for kf in &mut track.keyframes {
                                kf.tick += offset;
                            }
                        }
                    }

                    p_id = next_id;
                    all_tracks.extend(tracks);
                }
                None => {
                    errors.push(format!("Failed to parse: {}", line));
                }
            }
        }
    }

    if all_tracks.is_empty() {
        return Err(if errors.is_empty() {
            "No particles generated".into()
        } else {
            errors.join("\n")
        });
    }

    let frames = tracks_to_frames(&all_tracks);
    Ok((frames, 60))
}

/// Validate a command line. Returns Ok(description) or Err(error message).
pub fn validate_command(line: &str) -> Result<String, String> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err("Empty command".into());
    }
    let lower = trimmed.to_lowercase();
    if !lower.starts_with("particleex")
        && !lower.starts_with("/particleex")
        && !lower.starts_with("particlex")
        && !lower.starts_with("/particlex")
    {
        return Err("Command must start with particleex or particlex".into());
    }
    match parse_command(trimmed) {
        Some(cmd) => {
            let mode = &cmd.type_name;
            let lifespan = cmd.lifespan;
            let info = if cmd.config.is_normal {
                format!("✅ {} | count={} lifespan={}", mode, cmd.count, lifespan)
            } else if cmd.config.is_conditional {
                format!(
                    "✅ {} | range={:.1}×{:.1}×{:.1} lifespan={}",
                    mode,
                    cmd.range[0] * 2.0,
                    cmd.range[1] * 2.0,
                    cmd.range[2] * 2.0,
                    lifespan
                )
            } else {
                let total =
                    ((cmd.t_end - cmd.t_begin) / cmd.t_step.abs().max(0.0001)).floor() as u32 + 1;
                format!(
                    "✅ {} | t=[{:.1}..{:.1}] particles≈{} lifespan={}",
                    mode, cmd.t_begin, cmd.t_end, total, lifespan
                )
            };
            Ok(info)
        }
        None => Err("❌ Failed to parse command arguments".into()),
    }
}
