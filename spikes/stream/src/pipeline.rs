//! Pipeline stream su DataFusion (PIANO 1.5-1.8).
//!
//! Il SQL si pianifica una volta. DataFusion valuta, a ogni micro-lotto, filtro, proiezione, chiavi dei gruppi
//! e argomenti degli aggregati (stadio di ingresso); lo stato per (finestra, gruppo) e' lo stato degli
//! accumulatori `GroupsAccumulator` di DataFusion, mai gli eventi grezzi. Alla chiusura di una finestra
//! DataFusion valuta HAVING e proiezione (stadio finale). topictoast tiene watermark, assegnazione alle
//! finestre e chiusura: nessun orologio di sistema qui dentro, il tempo arriva solo dal log (PIANO 1.9).
//!
//! Regole (`wm` = watermark, `L` = ritardo tollerato, `size`/`slide` = finestra), identiche nell'oracolo
//! (`check.rs`), che le riscrive senza DataFusion:
//! - blocco con arrivo `A`: un evento con `t > A + futuro` e' scartato e contato, non muove `wm`;
//!   una finestra `[s, s+size)` e' chiusa se `s < wm - size - L` (cioe' `wm > s + size + L`, con `wm` quello
//!   che c'era PRIMA del blocco); un evento e' «in ritardo» se la sua prima finestra e' chiusa, ed e'
//!   scartato se le sue finestre sono tutte chiuse; dopo il blocco `wm = max(wm, max t non futuri)`,
//!   `ultimo_wm = wm`, `ultimo_arrivo = A`;
//! - segnale con arrivo `S`: se `S - ultimo_arrivo >= quiete`, `wm = max(wm, ultimo_wm + (S - ultimo_arrivo))`;
//! - a fine micro-lotto si chiudono (ed emettono) le finestre con `s < wm - size - L`.
//!
//! Posizione e deduplicazione sono per pipeline (parte 2a): ogni `PipeState` porta la propria posizione
//! applicata (`applied`) e le chiavi dei blocchi viste (`seen`); una pipeline sospesa e ripresa rilegge il log dalla
//! propria posizione e ognuna salta quello che ha gia' applicato.

// spike: la forma a due rami (senza finestra, con finestra) e' piu' leggibile cosi'
#![allow(clippy::unnecessary_unwrap, clippy::needless_range_loop)]

use async_trait::async_trait;
use datafusion::arrow::array::*;
use datafusion::arrow::compute::{cast, concat_batches, filter_record_batch, take};
use datafusion::arrow::datatypes::{DataType, Field, Schema, SchemaRef, TimeUnit};
use datafusion::arrow::ipc::reader::StreamReader;
use datafusion::arrow::ipc::writer::StreamWriter;
use datafusion::arrow::record_batch::RecordBatch;
use datafusion::arrow::row::{RowConverter, SortField};
use datafusion::arrow::util::display::array_value_to_string;
use datafusion::catalog::{Session, TableProvider};
use datafusion::common::tree_node::{Transformed, TreeNode, TreeNodeRecursion};
use datafusion::datasource::{MemTable, TableType, provider_as_source};
use datafusion::execution::SessionStateBuilder;
use datafusion::execution::disk_manager::{DiskManagerBuilder, DiskManagerMode};
use datafusion::execution::object_store::ObjectStoreRegistry;
use datafusion::execution::runtime_env::RuntimeEnvBuilder;
use datafusion::logical_expr::{
    AggregateUDF, EmitTo, Expr, GroupsAccumulator, LogicalPlan, LogicalPlanBuilder, col,
};
use datafusion::object_store::ObjectStore;
use datafusion::physical_expr::aggregate::{AggregateExprBuilder, AggregateFunctionExpr};
use datafusion::physical_expr::expressions::Column as PCol;
use datafusion::physical_expr::{GroupsAccumulatorAdapter, PhysicalExpr};
use datafusion::physical_plan::{ExecutionPlan, collect};
use datafusion::prelude::{SQLOptions, SessionConfig, SessionContext};
use datafusion::sql::parser::Statement as DfStatement;
use datafusion::sql::sqlparser::ast::Statement;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use crate::dialect::{Barriers, Branch, DErr, analyse, derr};
use crate::spec::{PipelineDef, StreamDef};
use crate::wire::Msg;

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Riga destinata a un avvio sul motore: la chiave e' la chiave di idempotenza C-4.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Row {
    pub key: String,
    pub body: Value,
    /// stream di destinazione (C7); `None` = avvio di workflow
    #[serde(default)]
    pub dest: Option<String>,
}

/// Tabella i cui dati si sostituiscono a ogni lotto (colonne dello stream piu' quelle interne).
#[derive(Debug)]
pub struct Shared {
    schema: SchemaRef,
    data: Mutex<Vec<RecordBatch>>,
}

impl Shared {
    pub fn new(schema: SchemaRef) -> Arc<Shared> {
        Arc::new(Shared {
            schema,
            data: Mutex::new(vec![]),
        })
    }
    pub fn set(&self, b: RecordBatch) {
        *self.data.lock().unwrap() = vec![b];
    }
}

#[async_trait]
impl TableProvider for Shared {
    fn schema(&self) -> SchemaRef {
        self.schema.clone()
    }
    fn table_type(&self) -> TableType {
        TableType::Base
    }
    async fn scan(
        &self,
        state: &dyn Session,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> datafusion::common::Result<Arc<dyn ExecutionPlan>> {
        let b = self.data.lock().unwrap().clone();
        MemTable::try_new(self.schema.clone(), vec![b])?
            .scan(state, projection, filters, limit)
            .await
    }
}

/// Nessun archivio di oggetti, nemmeno `file://` (PIANO 1.7 strato 3).
#[derive(Debug)]
struct NoStores;
impl ObjectStoreRegistry for NoStores {
    fn register_store(
        &self,
        _: &url::Url,
        _: Arc<dyn ObjectStore>,
    ) -> Option<Arc<dyn ObjectStore>> {
        None
    }
    fn get_store(&self, u: &url::Url) -> datafusion::common::Result<Arc<dyn ObjectStore>> {
        Err(datafusion::common::DataFusionError::Plan(format!(
            "nessun archivio di oggetti: {u}"
        )))
    }
}

fn ts_type() -> DataType {
    DataType::Timestamp(TimeUnit::Millisecond, Some("UTC".into()))
}

/// Contesto di DataFusion secondo le barriere. Barriera 3 accesa: nessun archivio di oggetti (nemmeno `file://`),
/// nessuna funzione di tabella, nessun formato di file, nessuna fabbrica di tabelle, versamento su disco spento.
/// Barriera 3 spenta (solo controllo positivo): le impostazioni predefinite di DataFusion. Barriera 4 accesa: pool
/// di memoria con limite `mem_limit`.
pub fn context(b: &Barriers, mem_limit: usize) -> Res<SessionContext> {
    let mut rt = RuntimeEnvBuilder::new();
    if b.registration {
        rt = rt
            .with_object_store_registry(Arc::new(NoStores))
            .with_disk_manager_builder(
                DiskManagerBuilder::default().with_mode(DiskManagerMode::Disabled),
            );
    }
    if b.memory {
        rt = rt.with_memory_limit(mem_limit, 1.0);
    }
    let rt = rt.build_arc()?;
    let cfg = SessionConfig::new()
        .with_target_partitions(1)
        .with_information_schema(false)
        .with_batch_size(8192);
    let mut sb = SessionStateBuilder::new()
        .with_config(cfg)
        .with_runtime_env(rt)
        .with_default_features();
    if b.registration {
        sb = sb
            .with_table_functions(HashMap::new())
            .with_table_factories(HashMap::new())
            .with_file_formats(vec![]);
    }
    Ok(SessionContext::new_with_state(sb.build()))
}

/// Dove va ogni colonna dell'uscita dell'`Aggregate`.
enum Slot {
    Ws,
    We,
    Group(usize),
    Aggr(usize),
}

struct AggSpec {
    expr: Arc<AggregateFunctionExpr>,
    /// colonne degli argomenti nell'uscita dello stadio di ingresso
    cols: Vec<usize>,
}

enum Kind {
    Stateless,
    Window {
        size: i64,
        slide: i64,
        aggs: Vec<AggSpec>,
        /// colonne dei gruppi (non finestra) nell'uscita dello stadio di ingresso: 0..ng
        ng: usize,
        ts_col: usize,
        lb_col: usize,
        layout: Vec<Slot>,
        agg_schema: SchemaRef,
        conv: RowConverter,
        distinct: bool,
    },
}

pub struct Pipe {
    pub def: PipelineDef,
    pub stream: String,
    /// indice del ramo (`UNION ALL`) e numero di rami
    pub branch: usize,
    pub nbranches: usize,
    ctx: SessionContext,
    sin: Arc<Shared>,
    sout: Arc<Shared>,
    ext_schema: SchemaRef,
    stage_plan: LogicalPlan,
    final_plan: LogicalPlan,
    ts_idx: usize,
    kind: Kind,
}

impl Pipe {
    /// Costruisce la pipeline: un `Pipe` per ramo di `UNION ALL` (uno solo se e' una `SELECT`). Barriera 1: controllo
    /// sull'albero in `analyse`; barriera 2: `SQLOptions` sul piano logico; barriere 3 e 4: `context`.
    pub async fn build(
        def: &PipelineDef,
        streams: &[StreamDef],
        mem_limit: usize,
        b: &Barriers,
    ) -> Result<Vec<Pipe>, DErr> {
        let branches = analyse(def, streams, b.syntax)?;
        let n = branches.len();
        let mut out = vec![];
        for (k, br) in branches.into_iter().enumerate() {
            out.push(Pipe::build_branch(def, streams, mem_limit, b, k, n, br).await?);
        }
        Ok(out)
    }

    async fn build_branch(
        def: &PipelineDef,
        streams: &[StreamDef],
        mem_limit: usize,
        bar: &Barriers,
        branch: usize,
        nbranches: usize,
        br: Branch,
    ) -> Result<Pipe, DErr> {
        let sname = br.stream.clone();
        let sdef = streams
            .iter()
            .find(|s| s.name == sname)
            .ok_or_else(|| derr("unknown_stream", format!("stream sconosciuto: {sname}")))?;
        let win = br.window;
        // il tempo dell'evento serve solo alle pipeline con finestra
        let ts_name = sdef.event_time.clone().unwrap_or_default();
        let ts_idx = sdef
            .columns
            .iter()
            .position(|c| c.name == ts_name)
            .unwrap_or(0);
        if win.is_some() && sdef.event_time.is_none() {
            return Err(derr(
                "event_time_required",
                "una pipeline con finestra vuole lo stream con il tempo dell'evento",
            ));
        }
        // schema esteso: colonne dello stream + colonne virtuali per rendere pianificabile il SQL
        let mut fields: Vec<Field> = sdef
            .schema()
            .fields()
            .iter()
            .map(|f| f.as_ref().clone())
            .collect();
        for n in ["window_start", "window_end"] {
            fields.push(Field::new(n, ts_type(), true));
        }
        for n in ["__lb", "__blk", "__row"] {
            fields.push(Field::new(n, DataType::Int64, true));
        }
        let ext_schema: SchemaRef = Arc::new(Schema::new(fields));
        let ctx = context(bar, mem_limit)?;
        let sin = Shared::new(ext_schema.clone());
        ctx.register_table(&sname, sin.clone())?;
        let state = ctx.state();
        // il piano nasce dall'albero riscritto, mai dal testo
        let plan = state
            .statement_to_plan(DfStatement::Statement(Box::new(Statement::Query(
                Box::new(br.query),
            ))))
            .await?;
        if bar.options {
            SQLOptions::new()
                .with_allow_ddl(false)
                .with_allow_dml(false)
                .with_allow_statements(false)
                .verify_plan(&plan)?;
        }
        // coercizione dei tipi degli argomenti (es. avg su interi) fatta qui, perche' gli accumulatori
        // fisici si costruiscono a mano a partire dagli argomenti dell'Aggregate
        let plan = state
            .analyzer()
            .execute_and_check(plan, state.config_options(), |_, _| {})?;

        // sbuccia: [Distinct] Projection [Filter] Aggregate ...
        let (distinct, top) = match &plan {
            LogicalPlan::Distinct(d) => (true, d.input().as_ref().clone()),
            p => (false, p.clone()),
        };
        let LogicalPlan::Projection(proj) = &top else {
            return Err(derr(
                "plan_unexpected",
                "piano inatteso: manca la proiezione",
            ));
        };
        let blk = |p: LogicalPlan| -> Res<LogicalPlan> {
            let LogicalPlan::Projection(pr) = &p else {
                unreachable!()
            };
            let mut e = pr.expr.clone();
            e.push(col("__blk"));
            e.push(col("__row"));
            Ok(LogicalPlanBuilder::from(pr.input.as_ref().clone())
                .project(e)?
                .build()?)
        };
        let sout;
        let (stage_plan, final_plan, kind) = if win.is_none() {
            // senza finestra: filtra e proietta evento per evento
            if find_aggregate(&top).is_some() || distinct {
                return Err(derr(
                    "plan_unexpected",
                    "senza finestra niente aggregati ne' DISTINCT",
                ));
            }
            sout = Shared::new(ext_schema.clone());
            let sp = blk(top.clone())?;
            (sp.clone(), sp, Kind::Stateless)
        } else {
            let (size, slide) = win.unwrap();
            let agg = find_aggregate(&top)
                .ok_or_else(|| derr("plan_unexpected", "la query con finestra vuole GROUP BY"))?
                .clone();
            // schema dei gruppi/aggregati e disposizione delle colonne dell'Aggregate
            let mut layout = vec![];
            let mut gexprs = vec![];
            let (mut seen_ws, mut seen_we) = (false, false);
            for e in &agg.group_expr {
                match e {
                    Expr::Column(c) if c.name == "window_start" => {
                        seen_ws = true;
                        layout.push(Slot::Ws)
                    }
                    Expr::Column(c) if c.name == "window_end" => {
                        seen_we = true;
                        layout.push(Slot::We)
                    }
                    e => {
                        layout.push(Slot::Group(gexprs.len()));
                        gexprs.push(e.clone());
                    }
                }
            }
            if !(seen_ws && seen_we) {
                return Err(derr("plan_unexpected", "finestra mancante nel GROUP BY"));
            }
            // stadio di ingresso: [gruppi..., argomenti degli aggregati..., __ts, __lb]
            let mut exprs: Vec<Expr> = gexprs
                .iter()
                .enumerate()
                .map(|(k, e)| e.clone().alias(format!("g{k}")))
                .collect();
            let ng = exprs.len();
            let mut agg_defs: Vec<(Arc<AggregateUDF>, bool, usize, usize)> = vec![];
            for (j, a) in agg.aggr_expr.iter().enumerate() {
                let mut a = a;
                while let Expr::Alias(al) = a {
                    a = &al.expr;
                }
                let Expr::AggregateFunction(f) = a else {
                    return Err(derr(
                        "plan_unexpected",
                        "espressione aggregata non riconosciuta",
                    ));
                };
                if f.params.filter.is_some() || !f.params.order_by.is_empty() {
                    return Err(derr(
                        "plan_unexpected",
                        "FILTER e ORDER BY negli aggregati non ammessi",
                    ));
                }
                let first = exprs.len();
                for (k, arg) in f.params.args.iter().enumerate() {
                    exprs.push(arg.clone().alias(format!("a{j}_{k}")));
                }
                agg_defs.push((
                    f.func.clone(),
                    f.params.distinct,
                    first,
                    f.params.args.len(),
                ));
            }
            let ts_col = exprs.len();
            exprs.push(col(&ts_name).alias("__ts"));
            let lb_col = exprs.len();
            exprs.push(col("__lb"));
            let stage = LogicalPlanBuilder::from(agg.input.as_ref().clone())
                .project(exprs)?
                .build()?;
            let stage_schema: SchemaRef = Arc::new(stage.schema().as_arrow().clone());
            let mut aggs = vec![];
            for (j, (f, dist, first, n)) in agg_defs.into_iter().enumerate() {
                let args: Vec<Arc<dyn PhysicalExpr>> = (first..first + n)
                    .map(|i| {
                        Arc::new(PCol::new(stage_schema.field(i).name(), i))
                            as Arc<dyn PhysicalExpr>
                    })
                    .collect();
                let expr = Arc::new(
                    AggregateExprBuilder::new(f, args)
                        .schema(stage_schema.clone())
                        .alias(format!("agg{j}"))
                        .with_distinct(dist)
                        .build()?,
                );
                aggs.push(AggSpec {
                    expr,
                    cols: (first..first + n).collect(),
                });
            }
            // slot degli aggregati dopo i gruppi
            for j in 0..aggs.len() {
                layout.push(Slot::Aggr(j));
            }
            // stadio finale: l'Aggregate diventa una tabella con lo stesso schema
            let agg_schema: SchemaRef = Arc::new(agg.schema.as_arrow().clone());
            sout = Shared::new(agg_schema.clone());
            let scan = LogicalPlanBuilder::scan(&sname, provider_as_source(sout.clone()), None)?
                .build()?;
            let new_input = proj
                .input
                .as_ref()
                .clone()
                .transform_down(|n| match n {
                    LogicalPlan::Aggregate(_) => Ok(Transformed::new(
                        scan.clone(),
                        true,
                        TreeNodeRecursion::Jump,
                    )),
                    n => Ok(Transformed::no(n)),
                })?
                .data;
            let mut fexprs = proj.expr.clone();
            if !distinct {
                // colonne nascoste con la chiave del gruppo (C-4)
                let cols = agg.schema.columns();
                let mut gi = 0;
                for (i, e) in agg.group_expr.iter().enumerate() {
                    if matches!(e, Expr::Column(c) if c.name == "window_start" || c.name == "window_end")
                    {
                        continue;
                    }
                    fexprs.push(Expr::Column(cols[i].clone()).alias(format!("__gk{gi}")));
                    gi += 1;
                }
            }
            let fin = LogicalPlanBuilder::from(new_input)
                .project(fexprs)?
                .build()?;
            // convertitore delle chiavi dei gruppi (stesse colonne dello stadio di ingresso)
            let conv = RowConverter::new(
                (0..ng)
                    .map(|i| SortField::new(stage_schema.field(i).data_type().clone()))
                    .collect(),
            )?;
            (
                stage,
                fin,
                Kind::Window {
                    size,
                    slide,
                    aggs,
                    ng,
                    ts_col,
                    lb_col,
                    layout,
                    agg_schema,
                    conv,
                    distinct,
                },
            )
        };
        Ok(Pipe {
            def: def.clone(),
            stream: sname,
            branch,
            nbranches,
            ctx,
            sin,
            sout,
            ext_schema,
            stage_plan,
            final_plan,
            ts_idx,
            kind,
        })
    }

    /// Prefisso delle chiavi C-4 di questa pipeline (con il ramo se e' un `UNION ALL`).
    fn kp(&self) -> String {
        if self.nbranches > 1 {
            format!("{}:{}:b{}", self.def.id, self.def.version, self.branch)
        } else {
            format!("{}:{}", self.def.id, self.def.version)
        }
    }

    /// Stato vuoto della pipeline, che parte dalla posizione `from` del log.
    pub fn new_state(&self, from: u64) -> PipeState {
        PipeState {
            applied: from,
            ..PipeState::default()
        }
    }

    /// Byte di stato contati (quota V3): somma di `GroupsAccumulator::size()` di ogni accumulatore, piu' le chiavi dei
    /// gruppi (tenute due volte, vettore e mappa, con un costo fisso per voce) e le chiavi dei blocchi viste.
    /// ponytail: stima, non misura di allocatore; basta a fermare una query ostile prima del limite del sistema.
    pub fn state_bytes(&self, st: &PipeState) -> usize {
        let wins: usize = st
            .wins
            .values()
            .map(|w| w.accs.iter().map(|a| a.size()).sum::<usize>() + w.key_bytes)
            .sum();
        wins + st.seen_bytes
    }

    /// Elabora i messaggi nuovi di un micro-lotto, nell'ordine del log. Salta cio' che questa pipeline ha gia'
    /// applicato (`off < applied`) e i blocchi con una chiave gia' vista. Torna le righe delle finestre chiuse (o le
    /// righe del filtro).
    pub async fn process(&self, st: &mut PipeState, msgs: &[(u64, &Msg)]) -> Res<Vec<Row>> {
        let (lateness, future, quiet) = (
            self.def.lateness_s * 1000,
            self.def.future_s * 1000,
            self.def.quiet_s * 1000,
        );
        let (size, slide) = match &self.kind {
            Kind::Window { size, slide, .. } => (*size, *slide),
            Kind::Stateless => (0, 0),
        };
        let mut parts: Vec<RecordBatch> = vec![];
        for (off, m) in msgs {
            if *off < st.applied {
                continue;
            }
            match m {
                Msg::Signal { arrival } => {
                    if let Some(wm) = st.wm
                        && arrival - st.last_arrival >= quiet
                    {
                        st.wm = Some(wm.max(st.last_block_wm + (arrival - st.last_arrival)));
                    }
                }
                Msg::Block {
                    key,
                    batch: Some(b),
                    arrival,
                    ..
                } => {
                    if !st.seen.insert(key.clone()) {
                        st.dup_blocks += 1;
                        continue;
                    }
                    st.seen_bytes += key.len() + 56;
                    let n = b.num_rows();
                    let mut lbv = i64::MIN;
                    let mut keep = vec![true; n];
                    if matches!(self.kind, Kind::Window { .. }) {
                        let ts = b
                            .column(self.ts_idx)
                            .as_any()
                            .downcast_ref::<TimestampMillisecondArray>()
                            .unwrap();
                        if let Some(w) = st.wm {
                            lbv = w - size - lateness;
                        }
                        let mut maxt: Option<i64> = None;
                        for i in 0..n {
                            let t = ts.value(i);
                            if t > arrival + future {
                                st.future += 1;
                                keep[i] = false;
                                continue;
                            }
                            maxt = Some(maxt.map_or(t, |m| m.max(t)));
                            let s_last = t.div_euclid(slide) * slide;
                            let s_first = (t - size).div_euclid(slide) * slide + slide;
                            if s_first < lbv {
                                st.late += 1;
                            }
                            keep[i] = s_last >= lbv;
                        }
                        if let Some(m) = maxt {
                            st.wm = Some(st.wm.map_or(m, |w| w.max(m)));
                        }
                        st.last_block_wm = st.wm.unwrap_or(0);
                        st.last_arrival = *arrival;
                    }
                    // batch esteso: colonne dello stream + virtuali + interne
                    let mut cols: Vec<ArrayRef> = b.columns().to_vec();
                    cols.push(new_null_array(&ts_type(), n));
                    cols.push(new_null_array(&ts_type(), n));
                    cols.push(Arc::new(Int64Array::from(vec![lbv; n])));
                    cols.push(Arc::new(Int64Array::from(vec![*off as i64; n])));
                    cols.push(Arc::new(Int64Array::from_iter_values(0..n as i64)));
                    let ext = RecordBatch::try_new(self.ext_schema.clone(), cols)?;
                    let mask = BooleanArray::from(keep);
                    let ext = filter_record_batch(&ext, &mask)?;
                    if ext.num_rows() > 0 {
                        parts.push(ext);
                    }
                }
                Msg::Block { .. } => {}
            }
        }
        if let Some((last, _)) = msgs.last() {
            st.applied = st.applied.max(last + 1);
        }
        let mut rows = vec![];
        if !parts.is_empty() {
            self.sin.set(concat_batches(&self.ext_schema, &parts)?);
            let phys = self
                .ctx
                .state()
                .create_physical_plan(&self.stage_plan)
                .await?;
            let out = collect(phys, self.ctx.task_ctx()).await?;
            match &self.kind {
                Kind::Stateless => {
                    for b in &out {
                        self.stateless_rows(b, &mut rows);
                    }
                }
                Kind::Window { .. } => {
                    for b in &out {
                        self.add_to_windows(st, b)?;
                    }
                }
            }
        }
        if let (Kind::Window { .. }, Some(wm)) = (&self.kind, st.wm) {
            let lb = wm - size - lateness;
            let closing: Vec<i64> = st.wins.range(..lb).map(|(k, _)| *k).collect();
            for ws in closing {
                let win = st.wins.remove(&ws).unwrap();
                rows.extend(self.emit(ws, win).await?);
            }
        }
        Ok(rows)
    }

    fn stateless_rows(&self, b: &RecordBatch, rows: &mut Vec<Row>) {
        let nc = b.num_columns();
        let (blk, row) = (b.column(nc - 2), b.column(nc - 1));
        let kp = self.kp();
        for i in 0..b.num_rows() {
            let key = format!(
                "{kp}:{}:{}",
                array_value_to_string(blk, i).unwrap(),
                array_value_to_string(row, i).unwrap()
            );
            rows.push(Row {
                key,
                body: row_json(b, i, nc - 2),
                dest: self.def.dest_stream.clone(),
            });
        }
    }

    /// Assegna le righe dello stadio di ingresso alle finestre e aggiorna gli accumulatori.
    fn add_to_windows(&self, st: &mut PipeState, b: &RecordBatch) -> Res<()> {
        let Kind::Window {
            size,
            slide,
            aggs,
            ng,
            ts_col,
            lb_col,
            conv,
            ..
        } = &self.kind
        else {
            unreachable!()
        };
        let (size, slide) = (*size, *slide);
        let ts = b
            .column(*ts_col)
            .as_any()
            .downcast_ref::<TimestampMillisecondArray>()
            .unwrap();
        let lbs = b
            .column(*lb_col)
            .as_any()
            .downcast_ref::<Int64Array>()
            .unwrap();
        let keys: Option<datafusion::arrow::row::Rows> = if *ng > 0 {
            Some(conv.convert_columns(&b.columns()[..*ng])?)
        } else {
            None
        };
        let mut per_win: BTreeMap<i64, Vec<u32>> = BTreeMap::new();
        for i in 0..b.num_rows() {
            let t = ts.value(i);
            let lb = lbs.value(i);
            let s_last = t.div_euclid(slide) * slide;
            let mut s = (t - size).div_euclid(slide) * slide + slide;
            while s <= s_last {
                if s >= lb {
                    per_win.entry(s).or_default().push(i as u32);
                }
                s += slide;
            }
        }
        for (ws, idxs) in per_win {
            let win = st.wins.entry(ws).or_insert_with(|| self.new_win());
            let mut gidx = Vec::with_capacity(idxs.len());
            for &i in &idxs {
                let row;
                let kb: &[u8] = match &keys {
                    Some(k) => {
                        row = k.row(i as usize);
                        row.as_ref()
                    }
                    None => &[],
                };
                let gi = match win.map.get(kb) {
                    Some(g) => *g,
                    None => {
                        win.keys.push(kb.to_vec());
                        win.key_bytes += 2 * kb.len() + 80;
                        win.map.insert(kb.to_vec(), win.keys.len() - 1);
                        win.keys.len() - 1
                    }
                };
                gidx.push(gi);
            }
            let total = win.keys.len();
            let ia = UInt32Array::from(idxs);
            for (j, a) in aggs.iter().enumerate() {
                let args: Vec<ArrayRef> = a
                    .cols
                    .iter()
                    .map(|c| take(b.column(*c).as_ref(), &ia, None))
                    .collect::<Result<_, _>>()?;
                win.accs[j].update_batch(&args, &gidx, None, total)?;
            }
        }
        Ok(())
    }

    fn new_win(&self) -> Win {
        let Kind::Window { aggs, .. } = &self.kind else {
            unreachable!()
        };
        Win {
            keys: vec![],
            key_bytes: 0,
            map: HashMap::new(),
            accs: aggs.iter().map(new_acc).collect(),
        }
    }

    /// Valori finali di una finestra chiusa, poi HAVING e proiezione con DataFusion.
    async fn emit(&self, ws: i64, mut win: Win) -> Res<Vec<Row>> {
        let Kind::Window {
            size,
            ng,
            layout,
            agg_schema,
            conv,
            distinct,
            ..
        } = &self.kind
        else {
            unreachable!()
        };
        let n = win.keys.len();
        let aggr: Vec<ArrayRef> = win
            .accs
            .iter_mut()
            .map(|a| a.evaluate(EmitTo::All))
            .collect::<Result<_, _>>()?;
        let groups: Vec<ArrayRef> = if *ng > 0 {
            let bin = BinaryArray::from_iter_values(win.keys.iter());
            let rows = conv.from_binary(bin);
            conv.convert_rows(rows.iter())?
        } else {
            vec![]
        };
        let tsa = |v: i64| -> ArrayRef {
            Arc::new(TimestampMillisecondArray::from(vec![v; n]).with_timezone("UTC"))
        };
        let mut cols: Vec<ArrayRef> = vec![];
        for (i, s) in layout.iter().enumerate() {
            let a = match s {
                Slot::Ws => tsa(ws),
                Slot::We => tsa(ws + size),
                Slot::Group(k) => groups[*k].clone(),
                Slot::Aggr(j) => aggr[*j].clone(),
            };
            let want = agg_schema.field(i).data_type();
            cols.push(if a.data_type() == want {
                a
            } else {
                cast(&a, want)?
            });
        }
        self.sout
            .set(RecordBatch::try_new(agg_schema.clone(), cols)?);
        let phys = self
            .ctx
            .state()
            .create_physical_plan(&self.final_plan)
            .await?;
        let out = collect(phys, self.ctx.task_ctx()).await?;
        let we = ws + size;
        let kp = self.kp();
        let mut rows = vec![];
        let mut seen: HashSet<String> = HashSet::new();
        for b in &out {
            let names: Vec<String> = b
                .schema()
                .fields()
                .iter()
                .map(|f| f.name().clone())
                .collect();
            let nvis = names.iter().filter(|n| !n.starts_with("__gk")).count();
            for i in 0..b.num_rows() {
                // chiave del gruppo: colonne nascoste; con DISTINCT, le colonne di uscita non di finestra
                let gk = if *distinct {
                    (0..nvis)
                        .filter(|c| names[*c] != "window_start" && names[*c] != "window_end")
                        .map(|c| array_value_to_string(b.column(c), i).unwrap())
                        .collect::<Vec<_>>()
                        .join("|")
                } else {
                    (nvis..names.len())
                        .map(|c| array_value_to_string(b.column(c), i).unwrap())
                        .collect::<Vec<_>>()
                        .join("|")
                };
                if *distinct && !seen.insert(gk.clone()) {
                    continue;
                }
                rows.push(Row {
                    key: format!("{kp}:{we}:{gk}"),
                    body: row_json(b, i, nvis),
                    dest: self.def.dest_stream.clone(),
                });
            }
        }
        Ok(rows)
    }

    /// Prova a secco (V3): esegue lo stadio di ingresso su un lotto sintetico di `rows` righe costruite dallo schema,
    /// per fare emergere gli errori di esecuzione (e i consumi) prima di salvare la pipeline.
    pub async fn dry_run(&self, sdef: &StreamDef, rows: usize) -> Result<(), DErr> {
        let n = rows;
        let mut cols: Vec<ArrayRef> = vec![];
        for c in &sdef.columns {
            let a: ArrayRef = match c.ty {
                crate::spec::ColType::Text => Arc::new(StringArray::from_iter_values(
                    (0..n).map(|i| format!("v{i}")),
                )),
                crate::spec::ColType::Int => Arc::new(Int64Array::from_iter_values(0..n as i64)),
                crate::spec::ColType::Decimal => {
                    Arc::new(Float64Array::from_iter_values((0..n).map(|i| i as f64)))
                }
                crate::spec::ColType::Bool => Arc::new(BooleanArray::from(vec![true; n])),
                crate::spec::ColType::Instant => Arc::new(
                    TimestampMillisecondArray::from_iter_values(
                        (0..n as i64).map(|i| 1_700_000_000_000 + i),
                    )
                    .with_timezone("UTC"),
                ),
            };
            cols.push(a);
        }
        cols.push(new_null_array(&ts_type(), n));
        cols.push(new_null_array(&ts_type(), n));
        cols.push(Arc::new(Int64Array::from(vec![i64::MIN; n])));
        cols.push(Arc::new(Int64Array::from_iter_values(0..n as i64)));
        cols.push(Arc::new(Int64Array::from_iter_values(0..n as i64)));
        self.sin.set(
            RecordBatch::try_new(self.ext_schema.clone(), cols)
                .map_err(|e| derr("plan_error", e.to_string()))?,
        );
        let phys = self
            .ctx
            .state()
            .create_physical_plan(&self.stage_plan)
            .await?;
        collect(phys, self.ctx.task_ctx()).await?;
        Ok(())
    }
}

fn new_acc(a: &AggSpec) -> Box<dyn GroupsAccumulator> {
    // alcuni (es. avg su Int64) dichiarano il supporto ma poi rifiutano: si ricade sull'adattatore
    match a
        .expr
        .groups_accumulator_supported()
        .then(|| a.expr.create_groups_accumulator())
    {
        Some(Ok(g)) => g,
        _ => {
            let e = a.expr.clone();
            Box::new(GroupsAccumulatorAdapter::new(move || {
                e.create_accumulator()
            }))
        }
    }
}

fn find_aggregate(p: &LogicalPlan) -> Option<&datafusion::logical_expr::Aggregate> {
    if let LogicalPlan::Aggregate(a) = p {
        return Some(a);
    }
    p.inputs().into_iter().find_map(find_aggregate)
}

/// Valore di una cella come JSON, con i tipi esatti (`3` resta `3`).
pub fn value_at(a: &ArrayRef, i: usize) -> Value {
    if a.is_null(i) {
        return Value::Null;
    }
    match a.data_type() {
        DataType::Int64 => Value::from(a.as_any().downcast_ref::<Int64Array>().unwrap().value(i)),
        DataType::UInt64 => Value::from(a.as_any().downcast_ref::<UInt64Array>().unwrap().value(i)),
        DataType::Int32 => Value::from(a.as_any().downcast_ref::<Int32Array>().unwrap().value(i)),
        DataType::Float64 => serde_json::Number::from_f64(
            a.as_any().downcast_ref::<Float64Array>().unwrap().value(i),
        )
        .map(Value::Number)
        .unwrap_or(Value::Null),
        DataType::Boolean => {
            Value::from(a.as_any().downcast_ref::<BooleanArray>().unwrap().value(i))
        }
        DataType::Utf8 => Value::from(a.as_any().downcast_ref::<StringArray>().unwrap().value(i)),
        DataType::Timestamp(TimeUnit::Millisecond, _) => Value::from(
            a.as_any()
                .downcast_ref::<TimestampMillisecondArray>()
                .unwrap()
                .value(i),
        ),
        _ => Value::from(array_value_to_string(a, i).unwrap_or_default()),
    }
}

fn row_json(b: &RecordBatch, i: usize, ncols: usize) -> Value {
    let mut m = Map::new();
    for c in 0..ncols {
        m.insert(b.schema().field(c).name().clone(), value_at(b.column(c), i));
    }
    Value::Object(m)
}

/// Una finestra aperta: chiavi dei gruppi (righe Arrow serializzate) e un accumulatore per aggregato.
pub struct Win {
    keys: Vec<Vec<u8>>,
    key_bytes: usize,
    map: HashMap<Vec<u8>, usize>,
    accs: Vec<Box<dyn GroupsAccumulator>>,
}

/// Stato di una pipeline (finestre, watermark, contatori, posizione applicata, chiavi dei blocchi viste); si salva
/// nel checkpoint.
#[derive(Default)]
pub struct PipeState {
    pub wm: Option<i64>,
    pub last_block_wm: i64,
    pub last_arrival: i64,
    pub late: u64,
    pub future: u64,
    pub wins: BTreeMap<i64, Win>,
    /// prossima posizione del log da applicare a questa pipeline
    pub applied: u64,
    /// chiavi dei blocchi gia' applicati (deduplicazione, PIANO 1.4). ponytail: cresce con lo stream e si salva
    /// intera nel checkpoint; il prodotto la pota con la conservazione.
    pub seen: HashSet<String>,
    pub seen_bytes: usize,
    pub dup_blocks: u64,
}

/// Intestazione serializzabile dello stato.
#[derive(Serialize, Deserialize, Default)]
pub struct PipeHead {
    pub id: String,
    pub version: u32,
    #[serde(default)]
    pub branch: usize,
    pub wm: Option<i64>,
    pub last_block_wm: i64,
    pub last_arrival: i64,
    pub late: u64,
    pub future: u64,
    pub applied: u64,
    pub seen: Vec<String>,
    pub dup_blocks: u64,
    /// (inizio finestra, gruppi)
    pub wins: Vec<(i64, usize)>,
}

fn ipc_bytes(b: &RecordBatch) -> Vec<u8> {
    let mut out = Vec::new();
    let mut w = StreamWriter::try_new(&mut out, &b.schema()).unwrap();
    w.write(b).unwrap();
    w.finish().unwrap();
    drop(w);
    out
}

impl Pipe {
    /// Salva lo stato: intestazione e un blocco Arrow IPC per finestra (chiavi + stato degli accumulatori).
    /// `GroupsAccumulator::state` svuota l'accumulatore: lo si rimette a posto subito con `merge_batch`.
    pub fn save(&self, st: &mut PipeState) -> Res<(PipeHead, Vec<Vec<u8>>)> {
        let mut head = PipeHead {
            id: self.def.id.clone(),
            version: self.def.version,
            branch: self.branch,
            wm: st.wm,
            last_block_wm: st.last_block_wm,
            last_arrival: st.last_arrival,
            late: st.late,
            future: st.future,
            applied: st.applied,
            seen: st.seen.iter().cloned().collect(),
            dup_blocks: st.dup_blocks,
            wins: vec![],
        };
        let mut blobs = vec![];
        for (ws, win) in st.wins.iter_mut() {
            let n = win.keys.len();
            let mut cols: Vec<ArrayRef> =
                vec![Arc::new(BinaryArray::from_iter_values(win.keys.iter()))];
            for a in win.accs.iter_mut() {
                let s = a.state(EmitTo::All)?;
                let idx: Vec<usize> = (0..n).collect();
                a.merge_batch(&s, &idx, n)?;
                cols.extend(s);
            }
            let fields: Vec<Field> = cols
                .iter()
                .enumerate()
                .map(|(i, c)| Field::new(format!("c{i}"), c.data_type().clone(), true))
                .collect();
            let b = RecordBatch::try_new(Arc::new(Schema::new(fields)), cols)?;
            head.wins.push((*ws, n));
            blobs.push(ipc_bytes(&b));
        }
        Ok((head, blobs))
    }

    pub fn restore(&self, head: &PipeHead, blobs: &[&[u8]]) -> Res<PipeState> {
        let mut st = PipeState {
            wm: head.wm,
            last_block_wm: head.last_block_wm,
            last_arrival: head.last_arrival,
            late: head.late,
            future: head.future,
            wins: BTreeMap::new(),
            applied: head.applied,
            seen_bytes: head.seen.iter().map(|k| k.len() + 56).sum(),
            seen: head.seen.iter().cloned().collect(),
            dup_blocks: head.dup_blocks,
        };
        for ((ws, n), blob) in head.wins.iter().zip(blobs) {
            let b = StreamReader::try_new(*blob, None)?
                .next()
                .ok_or("blocco di finestra vuoto")??;
            let mut win = self.new_win();
            let keys = b.column(0).as_any().downcast_ref::<BinaryArray>().unwrap();
            for i in 0..keys.len() {
                win.keys.push(keys.value(i).to_vec());
                win.key_bytes += 2 * keys.value(i).len() + 80;
                win.map.insert(keys.value(i).to_vec(), i);
            }
            let idx: Vec<usize> = (0..*n).collect();
            let mut c = 1;
            for a in win.accs.iter_mut() {
                let k = a.state(EmitTo::All)?.len(); // numero di colonne di stato (accumulatore vuoto)
                a.merge_batch(&b.columns()[c..c + k], &idx, *n)?;
                c += k;
            }
            st.wins.insert(*ws, win);
        }
        Ok(st)
    }
}
