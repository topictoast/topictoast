//! Dialetto chiuso sul albero sintattico (PIANO 1.6, prova V1) e interruttori delle barriere (prova V2).
//!
//! Il controllo lavora sull'AST di `sqlparser` (mai sul testo): una lista bianca di nodi, funzioni e aggregati; tutto
//! quello che non e' elencato e' rifiutato con un codice. `tumble(INTERVAL ..)` e `hop(..)` nel `GROUP BY` si
//! riconoscono e si riscrivono sull'albero in `window_start, window_end`; il testo non si ricompone mai.
//!
//! Ogni rifiuto ha un codice stabile (`DErr::code`). Un errore di sintassi porta riga e colonna.

use crate::spec::{PipelineDef, StreamDef};
use datafusion::sql::parser::{DFParserBuilder, Statement as DfStatement};
use datafusion::sql::sqlparser::ast::{
    BinaryOperator, CeilFloorKind, DataType, DateTimeField, Distinct, DuplicateTreatment, Expr,
    FunctionArg, FunctionArgExpr, FunctionArguments, GroupByExpr, Ident, ObjectNamePart, Query,
    Select, SelectItem, SetExpr, SetOperator, SetQuantifier, Statement, TableFactor, UnaryOperator,
    Value,
};
use datafusion::sql::sqlparser::dialect::GenericDialect;
use datafusion::sql::sqlparser::parser::ParserError;
use serde::Serialize;

pub const MAX_QUERY: usize = 16 << 10;
pub const MAX_LITERAL: usize = 1 << 10;
pub const MAX_DEPTH: usize = 64;
const MAX_BRANCHES: usize = 16;

/// Le quattro barriere sul SQL (PIANO 1.7). Il prodotto le usa sempre tutte (`Barriers::ALL`); ognuna si spegne
/// solo nei controlli positivi di V2 (`probe v2 --off ...`), che accetta lo spegnimento solo se il file di uscita ha
/// `-ctrl-` nel nome.
#[derive(Clone, Copy, Debug)]
pub struct Barriers {
    /// (1) controllo sull'albero sintattico
    pub syntax: bool,
    /// (2) `SQLOptions`: niente DDL, DML, istruzioni
    pub options: bool,
    /// (3) contesto senza nulla di registrato: niente archivi di oggetti, tabelle di funzione, formati, versamento
    pub registration: bool,
    /// (4) memoria: pool di DataFusion, quota di stato, limite del sistema
    pub memory: bool,
}

impl Barriers {
    pub const ALL: Barriers = Barriers {
        syntax: true,
        options: true,
        registration: true,
        memory: true,
    };
}

/// Errore con codice stabile. `line` e `column` solo per gli errori di sintassi.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DErr {
    pub code: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<u64>,
}

pub fn derr(code: &'static str, message: impl Into<String>) -> DErr {
    DErr {
        code,
        message: message.into(),
        line: None,
        column: None,
    }
}

impl std::fmt::Display for DErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl From<datafusion::error::DataFusionError> for DErr {
    fn from(e: datafusion::error::DataFusionError) -> DErr {
        let m = e.strip_backtrace();
        let code = if m.contains("Resources exhausted") || m.contains("memory") {
            "memory_exhausted"
        } else {
            "plan_error"
        };
        derr(code, m)
    }
}

impl From<datafusion::arrow::error::ArrowError> for DErr {
    fn from(e: datafusion::arrow::error::ArrowError) -> DErr {
        derr("plan_error", e.to_string())
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for DErr {
    fn from(e: Box<dyn std::error::Error + Send + Sync>) -> DErr {
        derr("plan_error", e.to_string())
    }
}

/// Un ramo del query (una `SELECT`): lo stream che legge, la `SELECT` riscritta e la finestra.
#[derive(Clone, Debug)]
pub struct Branch {
    pub stream: String,
    pub query: Query,
    /// (dimensione, scorrimento) in millisecondi
    pub window: Option<(i64, i64)>,
}

fn syntax_err(e: ParserError) -> DErr {
    let (msg, deep) = match &e {
        ParserError::TokenizerError(m) | ParserError::ParserError(m) => (m.clone(), false),
        ParserError::RecursionLimitExceeded => (String::new(), true),
    };
    if deep {
        return derr("too_deep", format!("annidamento oltre {MAX_DEPTH}"));
    }
    // i messaggi di sqlparser finiscono con `Line: N, Column: M`
    let pos = |tag: &str| {
        msg.split(tag)
            .nth(1)
            .and_then(|r| r.split(|c: char| !c.is_ascii_digit()).next())
            .and_then(|n| n.parse().ok())
    };
    DErr {
        code: "syntax_error",
        message: msg.clone(),
        line: pos("Line: "),
        column: pos("Column: "),
    }
}

/// Testo -> istruzioni. Il limite di ricorsione del parser (256) sta sopra il nostro (64): lo scatto di `too_deep`
/// spetta al controllo sull'albero; il limite del parser protegge solo lo stack.
pub fn parse(sql: &str) -> Result<std::collections::VecDeque<DfStatement>, DErr> {
    if sql.len() > MAX_QUERY {
        return Err(derr(
            "query_too_large",
            format!("query oltre {MAX_QUERY} byte"),
        ));
    }
    let d = GenericDialect {};
    DFParserBuilder::new(sql)
        .with_dialect(&d)
        .with_recursion_limit(256)
        .build()
        .and_then(|mut p| p.parse_statements())
        .map_err(|e| match e {
            datafusion::error::DataFusionError::SQL(pe, _) => syntax_err(*pe),
            other => syntax_err(ParserError::ParserError(other.strip_backtrace())),
        })
}

/// Nome corto del tipo di istruzione, per il messaggio di rifiuto.
fn statement_kind(s: &Statement) -> String {
    format!("{s:?}")
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_string()
}

fn interval_ms(e: &Expr) -> Result<i64, DErr> {
    let bad = || {
        derr(
            "window_invalid",
            "atteso INTERVAL '<n> <unita>' con second, minute o hour",
        )
    };
    let Expr::Interval(iv) = e else {
        return Err(bad());
    };
    if iv.last_field.is_some()
        || iv.leading_precision.is_some()
        || iv.fractional_seconds_precision.is_some()
    {
        return Err(bad());
    }
    let Expr::Value(v) = iv.value.as_ref() else {
        return Err(bad());
    };
    let (Value::SingleQuotedString(s) | Value::Number(s, _)) = &v.value else {
        return Err(bad());
    };
    let mut it = s.split_whitespace();
    let n: i64 = it.next().and_then(|x| x.parse().ok()).ok_or_else(bad)?;
    let unit = match (&iv.leading_field, it.next()) {
        (Some(DateTimeField::Second | DateTimeField::Seconds), None) => 1000,
        (Some(DateTimeField::Minute | DateTimeField::Minutes), None) => 60_000,
        (Some(DateTimeField::Hour | DateTimeField::Hours), None) => 3_600_000,
        (None, Some(u)) => match u.to_ascii_lowercase().as_str() {
            "second" | "seconds" => 1000,
            "minute" | "minutes" => 60_000,
            "hour" | "hours" => 3_600_000,
            _ => return Err(bad()),
        },
        _ => return Err(bad()),
    };
    if it.next().is_some() || n <= 0 || n > 10_000 {
        return Err(bad());
    }
    Ok(n * unit)
}

const SCALARS: [&str; 9] = [
    "coalesce",
    "lower",
    "upper",
    "length",
    "substr",
    "starts_with",
    "abs",
    "round",
    "ceil",
];
const AGGREGATES: [&str; 5] = ["count", "sum", "min", "max", "avg"];

struct Walk<'a> {
    /// `false` dentro `WHERE` e `GROUP BY`: gli aggregati non vi sono ammessi
    aggregates: bool,
    stream: &'a str,
}

impl Walk<'_> {
    fn ident(&self, i: &Ident) -> Result<(), DErr> {
        if i.value.starts_with("__") {
            return Err(derr("reserved_name", "nomi interni non ammessi"));
        }
        Ok(())
    }

    fn dtype(&self, t: &DataType) -> Result<(), DErr> {
        let s = t.to_string().to_ascii_lowercase();
        let base = s.split('(').next().unwrap_or("").trim();
        let ok = [
            "text",
            "varchar",
            "string",
            "bigint",
            "int64",
            "double",
            "double precision",
            "float64",
            "float8",
            "boolean",
            "bool",
            "timestamp",
        ];
        if ok.contains(&base) {
            Ok(())
        } else {
            Err(derr(
                "type_not_allowed",
                format!("CAST verso {s} non ammesso"),
            ))
        }
    }

    fn value(&self, v: &Value) -> Result<(), DErr> {
        if !matches!(
            v,
            Value::Number(..) | Value::SingleQuotedString(_) | Value::Boolean(_) | Value::Null
        ) {
            return Err(derr("literal_not_allowed", "letterale non ammesso"));
        }
        if v.to_string().len() > MAX_LITERAL {
            return Err(derr(
                "literal_too_large",
                format!("letterale oltre {MAX_LITERAL} byte"),
            ));
        }
        Ok(())
    }

    fn func(
        &self,
        f: &datafusion::sql::sqlparser::ast::Function,
        depth: usize,
    ) -> Result<(), DErr> {
        let name = match f.name.0.as_slice() {
            [ObjectNamePart::Identifier(i)] => i.value.to_ascii_lowercase(),
            _ => {
                return Err(derr(
                    "function_not_allowed",
                    "nome di funzione qualificato non ammesso",
                ));
            }
        };
        if f.over.is_some() {
            return Err(derr(
                "window_function_not_allowed",
                "funzioni finestra con OVER non ammesse",
            ));
        }
        if f.filter.is_some()
            || f.null_treatment.is_some()
            || !f.within_group.is_empty()
            || f.uses_odbc_syntax
            || !matches!(f.parameters, FunctionArguments::None)
        {
            return Err(derr(
                "function_not_allowed",
                format!("forma della chiamata a {name} non ammessa"),
            ));
        }
        let is_agg = AGGREGATES.contains(&name.as_str());
        if !is_agg && !SCALARS.contains(&name.as_str()) {
            let code = if name == "tumble" || name == "hop" {
                "window_misplaced"
            } else {
                "function_not_allowed"
            };
            return Err(derr(code, format!("funzione {name} non ammessa")));
        }
        if is_agg && !self.aggregates {
            return Err(derr(
                "aggregate_misplaced",
                format!("aggregato {name} non ammesso qui"),
            ));
        }
        let FunctionArguments::List(l) = &f.args else {
            return Err(derr(
                "function_not_allowed",
                format!("chiamata a {name} senza elenco di argomenti"),
            ));
        };
        if !l.clauses.is_empty() {
            return Err(derr(
                "function_not_allowed",
                "clausole negli argomenti non ammesse",
            ));
        }
        if l.duplicate_treatment == Some(DuplicateTreatment::Distinct) && name != "count" {
            return Err(derr("function_not_allowed", "DISTINCT solo in count"));
        }
        for a in &l.args {
            match a {
                FunctionArg::Unnamed(FunctionArgExpr::Expr(e)) => self.expr(e, depth + 1)?,
                FunctionArg::Unnamed(FunctionArgExpr::Wildcard) if name == "count" => {}
                _ => {
                    return Err(derr(
                        "function_not_allowed",
                        "forma dell'argomento non ammessa",
                    ));
                }
            }
        }
        Ok(())
    }

    fn expr(&self, e: &Expr, depth: usize) -> Result<(), DErr> {
        if depth > MAX_DEPTH {
            return Err(derr("too_deep", format!("annidamento oltre {MAX_DEPTH}")));
        }
        let d = depth + 1;
        match e {
            Expr::Identifier(i) => self.ident(i),
            Expr::CompoundIdentifier(p) => match p.as_slice() {
                [q, c] if q.value == self.stream => self.ident(c),
                _ => Err(derr(
                    "qualified_name_not_allowed",
                    "nome qualificato non ammesso",
                )),
            },
            Expr::Value(v) => self.value(&v.value),
            Expr::Nested(x)
            | Expr::IsNull(x)
            | Expr::IsNotNull(x)
            | Expr::IsTrue(x)
            | Expr::IsNotTrue(x)
            | Expr::IsFalse(x)
            | Expr::IsNotFalse(x) => self.expr(x, d),
            Expr::UnaryOp {
                op: UnaryOperator::Plus | UnaryOperator::Minus | UnaryOperator::Not,
                expr,
            } => self.expr(expr, d),
            Expr::BinaryOp { left, op, right } => {
                use BinaryOperator::*;
                if !matches!(
                    op,
                    Plus | Minus
                        | Multiply
                        | Divide
                        | Modulo
                        | Gt
                        | Lt
                        | GtEq
                        | LtEq
                        | Eq
                        | NotEq
                        | And
                        | Or
                ) {
                    return Err(derr(
                        "operator_not_allowed",
                        format!("operatore {op} non ammesso"),
                    ));
                }
                self.expr(left, d)?;
                self.expr(right, d)
            }
            Expr::Between {
                expr, low, high, ..
            } => {
                self.expr(expr, d)?;
                self.expr(low, d)?;
                self.expr(high, d)
            }
            Expr::InList { expr, list, .. } => {
                self.expr(expr, d)?;
                list.iter().try_for_each(|x| self.expr(x, d))
            }
            Expr::Like {
                expr,
                pattern,
                escape_char: None,
                any: false,
                ..
            } => {
                self.expr(expr, d)?;
                self.expr(pattern, d)
            }
            Expr::Cast {
                kind: datafusion::sql::sqlparser::ast::CastKind::Cast,
                expr,
                data_type,
                format: None,
                ..
            } => {
                self.dtype(data_type)?;
                self.expr(expr, d)
            }
            Expr::Case {
                operand,
                conditions,
                else_result,
                ..
            } => {
                if let Some(o) = operand {
                    self.expr(o, d)?;
                }
                for w in conditions {
                    self.expr(&w.condition, d)?;
                    self.expr(&w.result, d)?;
                }
                else_result.as_ref().map_or(Ok(()), |x| self.expr(x, d))
            }
            // `FLOOR(x)` e `CEIL(x)` sono nodi propri; `SUBSTR(a, b, c)` puo' esserlo
            Expr::Floor {
                expr,
                field: CeilFloorKind::DateTimeField(DateTimeField::NoDateTime),
            }
            | Expr::Ceil {
                expr,
                field: CeilFloorKind::DateTimeField(DateTimeField::NoDateTime),
            } => self.expr(expr, d),
            Expr::Substring {
                expr,
                substring_from,
                substring_for,
                ..
            } => {
                self.expr(expr, d)?;
                substring_from
                    .iter()
                    .chain(substring_for.iter())
                    .try_for_each(|x| self.expr(x, d))
            }
            Expr::Function(f) => self.func(f, d),
            Expr::Subquery(_) | Expr::InSubquery { .. } | Expr::Exists { .. } => {
                Err(derr("subquery_not_allowed", "sottoquery non ammesse"))
            }
            Expr::Wildcard(_) | Expr::QualifiedWildcard(..) => {
                Err(derr("wildcard_not_allowed", "wildcard non ammesso"))
            }
            other => Err(derr(
                "construct_not_allowed",
                format!("costrutto {} non ammesso", statement_kind_expr(other)),
            )),
        }
    }
}

fn statement_kind_df(s: &DfStatement) -> String {
    format!("{s:?}")
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_string()
}

fn statement_kind_expr(e: &Expr) -> String {
    format!("{e:?}")
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .next()
        .unwrap_or("")
        .to_string()
}

/// `tumble(INTERVAL ..)` o `hop(INTERVAL .., INTERVAL ..)` come elemento del `GROUP BY`.
fn window_call(e: &Expr) -> Option<Result<(i64, i64), DErr>> {
    let Expr::Function(f) = e else { return None };
    let [ObjectNamePart::Identifier(n)] = f.name.0.as_slice() else {
        return None;
    };
    let name = n.value.to_ascii_lowercase();
    if name != "tumble" && name != "hop" {
        return None;
    }
    let FunctionArguments::List(l) = &f.args else {
        return Some(Err(derr("window_invalid", "argomenti mancanti")));
    };
    let ivs: Result<Vec<i64>, DErr> = l
        .args
        .iter()
        .map(|a| match a {
            FunctionArg::Unnamed(FunctionArgExpr::Expr(x)) => interval_ms(x),
            _ => Err(derr("window_invalid", "atteso un intervallo")),
        })
        .collect();
    Some(ivs.and_then(|v| match (name.as_str(), v.as_slice()) {
        ("tumble", [s]) => Ok((*s, *s)),
        ("hop", [size, slide]) if size % slide == 0 && size >= slide => Ok((*size, *slide)),
        ("hop", [_, _]) => Err(derr(
            "window_invalid",
            "la dimensione deve essere multipla dello scorrimento",
        )),
        _ => Err(derr("window_invalid", "numero di argomenti non valido")),
    }))
}

fn select_branch(s: &Select, streams: &[StreamDef], check: bool) -> Result<Branch, DErr> {
    if s.from.len() != 1 {
        return Err(derr(
            "join_not_allowed",
            "una sola tabella nel FROM (niente prodotto cartesiano)",
        ));
    }
    let from = &s.from[0];
    if !from.joins.is_empty() {
        return Err(derr("join_not_allowed", "join non ammessi"));
    }
    let name = match &from.relation {
        TableFactor::Table {
            name,
            args: None,
            alias: None,
            ..
        } => match name.0.as_slice() {
            [ObjectNamePart::Identifier(i)] => i.value.clone(),
            _ => {
                return Err(derr(
                    "qualified_name_not_allowed",
                    "nome di stream qualificato non ammesso",
                ));
            }
        },
        TableFactor::Table { args: Some(_), .. }
        | TableFactor::Function { .. }
        | TableFactor::TableFunction { .. }
        | TableFactor::UNNEST { .. } => {
            return Err(derr(
                "table_function_not_allowed",
                "funzioni di tabella non ammesse",
            ));
        }
        TableFactor::Derived { .. } | TableFactor::NestedJoin { .. } => {
            return Err(derr("subquery_not_allowed", "sottoquery non ammesse"));
        }
        _ => {
            return Err(derr(
                "construct_not_allowed",
                "FROM: forma non ammessa (niente alias)",
            ));
        }
    };
    // uno stream di un altro tenant e uno inesistente danno lo stesso errore: qui si vedono solo gli stream del tenant
    if !streams.iter().any(|st| st.name == name) {
        return Err(derr(
            "unknown_stream",
            format!("stream sconosciuto: {name}"),
        ));
    }
    let mut window = None;
    if check {
        let q = s;
        if q.optimizer_hints.len()
            + q.lateral_views.len()
            + q.connect_by.len()
            + q.cluster_by.len()
            + q.distribute_by.len()
            + q.sort_by.len()
            + q.named_window.len()
            > 0
            || q.select_modifiers.is_some()
            || q.top.is_some()
            || q.into.is_some()
            || q.prewhere.is_some()
            || q.qualify.is_some()
            || q.value_table_mode.is_some()
            || q.exclude.is_some()
            || matches!(q.distinct, Some(Distinct::On(_)))
        {
            return Err(derr(
                "clause_not_allowed",
                "clausola non ammessa nella SELECT",
            ));
        }
        let w_all = Walk {
            aggregates: true,
            stream: &name,
        };
        let w_where = Walk {
            aggregates: false,
            stream: &name,
        };
        for it in &q.projection {
            match it {
                SelectItem::UnnamedExpr(e) => w_all.expr(e, 1)?,
                SelectItem::ExprWithAlias { expr, alias } => {
                    w_all.ident(alias)?;
                    w_all.expr(expr, 1)?
                }
                SelectItem::Wildcard(_) | SelectItem::QualifiedWildcard(..) => {
                    return Err(derr("wildcard_not_allowed", "wildcard non ammesso"));
                }
                _ => {
                    return Err(derr(
                        "construct_not_allowed",
                        "elemento della proiezione non ammesso",
                    ));
                }
            }
        }
        if let Some(e) = &q.selection {
            w_where.expr(e, 1)?;
        }
        if let Some(e) = &q.having {
            w_all.expr(e, 1)?;
        }
    }
    // finestre: riscrittura sull'albero del GROUP BY. La copia avviene dopo il controllo di profondita' (un albero profondo
    // migliaia di livelli non si copia: la ricorsione di `clone` lo farebbe traboccare)
    let mut q = if check {
        match &s.group_by {
            GroupByExpr::Expressions(v, mods) if mods.is_empty() => {
                for e in v {
                    if window_call(e).is_none() {
                        Walk {
                            aggregates: false,
                            stream: &name,
                        }
                        .expr(e, 1)?;
                    }
                }
            }
            _ => {
                return Err(derr(
                    "clause_not_allowed",
                    "GROUP BY: modificatori non ammessi",
                ));
            }
        }
        s.clone()
    } else {
        s.clone()
    };
    match &mut q.group_by {
        GroupByExpr::Expressions(v, mods) if mods.is_empty() => {
            let mut out = vec![];
            for e in v.drain(..) {
                match window_call(&e) {
                    Some(Ok(w)) => {
                        if window.replace(w).is_some() {
                            return Err(derr("window_invalid", "una sola finestra nel GROUP BY"));
                        }
                        out.push(Expr::Identifier(Ident::new("window_start")));
                        out.push(Expr::Identifier(Ident::new("window_end")));
                    }
                    Some(Err(x)) => return Err(x),
                    None => out.push(e),
                }
            }
            *v = out;
        }
        GroupByExpr::Expressions(..) | GroupByExpr::All(_) => {
            return Err(derr(
                "clause_not_allowed",
                "GROUP BY: modificatori non ammessi",
            ));
        }
    }
    Ok(Branch {
        stream: name,
        query: wrap(q),
        window,
    })
}

fn wrap(s: Select) -> Query {
    Query {
        with: None,
        body: Box::new(SetExpr::Select(Box::new(s))),
        order_by: None,
        limit_clause: None,
        fetch: None,
        locks: vec![],
        for_clause: None,
        settings: None,
        format_clause: None,
        pipe_operators: vec![],
    }
}

/// Appiattisce `SELECT .. UNION ALL SELECT ..` nei suoi rami.
fn leaves<'a>(b: &'a SetExpr, out: &mut Vec<&'a Select>, depth: usize) -> Result<(), DErr> {
    if depth > MAX_DEPTH || out.len() > MAX_BRANCHES {
        return Err(derr(
            "too_deep",
            "troppi rami o annidamento oltre il limite",
        ));
    }
    match b {
        SetExpr::Select(s) => {
            out.push(s);
            Ok(())
        }
        SetExpr::SetOperation {
            left,
            op: SetOperator::Union,
            set_quantifier: SetQuantifier::All,
            right,
        } => {
            leaves(left, out, depth + 1)?;
            leaves(right, out, depth + 1)
        }
        SetExpr::SetOperation { .. } => Err(derr("set_operation_not_allowed", "solo UNION ALL")),
        SetExpr::Query(_) => Err(derr("subquery_not_allowed", "sottoquery non ammesse")),
        _ => Err(derr("construct_not_allowed", "solo SELECT")),
    }
}

/// Analisi di una pipeline: parsing, controllo sull'albero (se la barriera 1 e' accesa), riscrittura delle finestre,
/// rami. La finestra oltre la conservazione dello stream e' rifiutata qui.
pub fn analyse(def: &PipelineDef, streams: &[StreamDef], check: bool) -> Result<Vec<Branch>, DErr> {
    let mut sts = parse(&def.sql)?;
    if sts.len() != 1 {
        return Err(derr("statement_not_allowed", "una sola istruzione"));
    }
    let q = match sts.pop_front().unwrap() {
        DfStatement::Statement(s) => match *s {
            Statement::Query(q) => q,
            other => {
                return Err(derr(
                    "statement_not_allowed",
                    format!("solo SELECT: {} non ammesso", statement_kind(&other)),
                ));
            }
        },
        other => {
            return Err(derr(
                "statement_not_allowed",
                format!("solo SELECT: {} non ammesso", statement_kind_df(&other)),
            ));
        }
    };
    if q.with.is_some() {
        return Err(derr("cte_not_allowed", "CTE non ammesse"));
    }
    if q.order_by.is_some() {
        return Err(derr("order_by_not_allowed", "ORDER BY non ammesso"));
    }
    if q.limit_clause.is_some() || q.fetch.is_some() {
        return Err(derr("limit_not_allowed", "LIMIT non ammesso"));
    }
    if !q.locks.is_empty()
        || q.for_clause.is_some()
        || q.settings.is_some()
        || q.format_clause.is_some()
        || !q.pipe_operators.is_empty()
    {
        return Err(derr("clause_not_allowed", "clausola non ammessa nel query"));
    }
    let mut ls = vec![];
    leaves(&q.body, &mut ls, 0)?;
    let branches: Vec<Branch> = ls
        .into_iter()
        .map(|s| select_branch(s, streams, check))
        .collect::<Result<_, _>>()?;
    for b in &branches {
        let sd = streams
            .iter()
            .find(|s| s.name == b.stream)
            .expect("stream gia' verificato");
        if let (Some((size, _)), Some(r)) = (b.window, sd.retention_s)
            && size + def.lateness_s * 1000 > r * 1000
        {
            return Err(derr(
                "window_exceeds_retention",
                format!(
                    "finestra piu' ritardo oltre la conservazione di {}",
                    sd.name
                ),
            ));
        }
    }
    Ok(branches)
}

/// Ciclo tra le pipeline di un tenant: ogni pipeline legge i suoi stream e scrive nello stream di destinazione.
/// `Some(catena)` se una pipeline puo' raggiungere se stessa.
pub fn find_cycle(defs: &[(String, Vec<String>, Option<String>)]) -> Option<Vec<String>> {
    // pipeline P -> pipeline Q se P scrive in uno stream che Q legge
    let next = |p: usize| -> Vec<usize> {
        defs.iter()
            .enumerate()
            .filter(|(_, q)| defs[p].2.as_ref().is_some_and(|d| q.1.contains(d)))
            .map(|(i, _)| i)
            .collect()
    };
    for start in 0..defs.len() {
        let mut stack = vec![(start, vec![start])];
        while let Some((p, path)) = stack.pop() {
            for q in next(p) {
                if q == start {
                    return Some(path.iter().map(|i| defs[*i].0.clone()).collect());
                }
                if !path.contains(&q) {
                    let mut np = path.clone();
                    np.push(q);
                    stack.push((q, np));
                }
            }
        }
    }
    None
}

/// Validazione dell'insieme di pipeline di un tenant: ogni pipeline e poi i cicli.
pub fn validate_set(
    defs: &[PipelineDef],
    streams: &[StreamDef],
    check: bool,
) -> Result<(), (String, DErr)> {
    let mut graph = vec![];
    for d in defs {
        let br = analyse(d, streams, check).map_err(|e| (d.id.clone(), e))?;
        if let Some(dst) = &d.dest_stream
            && !streams.iter().any(|s| &s.name == dst)
        {
            return Err((
                d.id.clone(),
                derr("unknown_stream", format!("stream sconosciuto: {dst}")),
            ));
        }
        graph.push((
            d.id.clone(),
            br.iter().map(|b| b.stream.clone()).collect(),
            d.dest_stream.clone(),
        ));
    }
    if let Some(c) = find_cycle(&graph) {
        return Err((
            c[0].clone(),
            derr(
                "pipeline_cycle",
                format!("ciclo tra le pipeline: {}", c.join(" -> ")),
            ),
        ));
    }
    Ok(())
}
