//! Spike: per-kit hash-map DP ("interpreter") vs traced Tape evaluated over lanes.
//! Synthetic mechanics only (no rules text): A attacks per turn; after a hit/crit an
//! optional "smite" spending one slot of any level (one slot per turn, uses the BA);
//! after the attack action, an optional BA "hew" attack if any crit was seen and BA unused;
//! a "vex" flag: a hit grants advantage on the next attack roll (persists across turns).
//! Objective: expected total damage over `horizon` rounds; we carry (E, E2) policy-aware.

use rustc_hash::FxHashMap;
use std::time::Instant;

const ACS: usize = 16; // AC 10..=25

#[derive(Clone)]
struct Shape {
    attacks: u8,
    slots: Vec<u8>, // max slots per level (len = levels)
    horizon: u8,
    hew: bool,
    vex: bool,
    canon: bool,  // canonicalise slots at turn boundaries: keep only the top-R slots (R = turns left)
    priced: bool, // Lagrangian relaxation: slots not counted, each use costs λ_l (term 12+l)
}

#[derive(Clone, Copy)]
struct Params {
    to_hit: i32,
    crit_on: i32,
    w_hit: (f64, f64),  // (E, E2) weapon damage on hit
    w_crit: (f64, f64), // on crit
    smite: [((f64, f64), (f64, f64)); 5], // per level: (hit, crit)
}

// ---------------- state ----------------
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct St {
    round: u8,
    atk: u8,
    phase: u8, // 0 attack, 1 smite-decide after hit, 2 smite-decide after crit, 3 hew-decide, 4 hew-attack
    slots: [u8; 5],
    slot_used: bool,
    ba_used: bool,
    crit_seen: bool,
    vex: bool,
}
impl St {
    fn key(&self) -> u64 {
        let mut k = self.round as u64;
        k = k << 4 | self.atk as u64;
        k = k << 3 | self.phase as u64;
        for s in self.slots { k = k << 3 | s as u64; }
        k = k << 1 | self.slot_used as u64;
        k = k << 1 | self.ba_used as u64;
        k = k << 1 | self.crit_seen as u64;
        k = k << 1 | self.vex as u64;
        k
    }
}

/// Structural successor description (shared by interpreter and tracer: ONE authority).
enum Step {
    Leaf,
    /// adv flag; outcomes miss/hit/crit -> (terms, next)
    Chance { adv: bool, out: [(Vec<u8>, St); 3] },
    Decide { opts: Vec<(Vec<u8>, St)> },
}
// term ids: 0 w_hit, 1 w_crit, 2+2l smite l hit, 3+2l smite l crit

fn canon_slots(sh: &Shape, mut s: St) -> St {
    if !sh.canon || s.round >= sh.horizon { return s; }
    let mut keep = (sh.horizon - s.round) as u8; // at most one slot per turn
    for l in (0..5).rev() { let k = s.slots[l].min(keep); keep -= k; s.slots[l] = k; }
    s
}
fn end_turn(sh: &Shape, s: St) -> St {
    canon_slots(sh, St { round: s.round + 1, atk: 0, phase: 0, slot_used: false, ba_used: false, crit_seen: false, ..s })
}

fn after_attack(sh: &Shape, s: St) -> St {
    let s = St { atk: s.atk + 1, phase: 0, ..s };
    if s.atk < sh.attacks { s } else { St { phase: 3, ..s } }
}

fn step(sh: &Shape, s: St) -> Step {
    if s.round >= sh.horizon { return Step::Leaf; }
    match s.phase {
        0 => {
            let adv = sh.vex && s.vex;
            let base = St { vex: false, ..s };
            let miss = after_attack(sh, base);
            let hit = St { phase: 1, vex: sh.vex, ..base };
            let crit = St { phase: 2, vex: sh.vex, crit_seen: true, ..base };
            Step::Chance { adv, out: [(vec![], miss), (vec![0], hit), (vec![1], crit)] }
        }
        1 | 2 => {
            let crit = s.phase == 2;
            let cont = after_attack(sh, s);
            let mut opts = vec![(vec![], cont)];
            if !s.slot_used && !s.ba_used {
                for l in 0..sh.slots.len() {
                    if sh.priced && sh.slots[l] > 0 {
                        let n = St { slot_used: true, ba_used: true, ..s };
                        let t = 2 + 2 * l as u8 + crit as u8;
                        opts.push((vec![t, 12 + l as u8], after_attack(sh, n)));
                    } else if s.slots[l] > 0 {
                        let mut n = s;
                        n.slots[l] -= 1;
                        n.slot_used = true;
                        n.ba_used = true;
                        let t = 2 + 2 * l as u8 + crit as u8;
                        opts.push((vec![t], after_attack(sh, n)));
                    }
                }
            }
            Step::Decide { opts }
        }
        3 => {
            let end = end_turn(sh, s);
            let mut opts = vec![(vec![], end)];
            if sh.hew && s.crit_seen && !s.ba_used {
                opts.push((vec![], St { phase: 4, ba_used: true, ..s }));
            }
            Step::Decide { opts }
        }
        4 => {
            let adv = sh.vex && s.vex;
            let base = St { vex: false, ..s };
            let e = end_turn(sh, base);
            let ev = end_turn(sh, St { vex: sh.vex, ..base });
            Step::Chance { adv, out: [(vec![], e), (vec![0], ev), (vec![1], ev)] }
        }
        _ => unreachable!(),
    }
}

fn root(sh: &Shape) -> St {
    let mut slots = [0u8; 5];
    if !sh.priced { for (i, &c) in sh.slots.iter().enumerate() { slots[i] = c; } }
    canon_slots(sh, St { round: 0, atk: 0, phase: 0, slots, slot_used: false, ba_used: false, crit_seen: false, vex: false })
}

// ---------------- d20 (exact integer counting, one division) ----------------
fn outcome(to_hit: i32, ac: i32, crit_on: i32, adv: bool) -> [f64; 3] {
    let t = (ac - to_hit).clamp(2, 20); // natural roll needed to hit
    let c = crit_on.min(20);
    let t = t.min(c);
    let (miss_n, crit_n, den) = if adv {
        let m = ((t - 1) * (t - 1)) as f64;
        let nc = ((c - 1) * (c - 1)) as f64;
        (m, 400.0 - nc, 400.0)
    } else {
        ((t - 1) as f64, (21 - c) as f64, 20.0)
    };
    let pm = miss_n / den;
    let pc = crit_n / den;
    [pm, 1.0 - pm - pc, pc]
}

fn term(p: &Params, t: u8) -> (f64, f64) {
    match t {
        0 => p.w_hit,
        1 => p.w_crit,
        _ => {
            let l = ((t - 2) / 2) as usize;
            if (t - 2) % 2 == 0 { p.smite[l].0 } else { p.smite[l].1 }
        }
    }
}
#[inline(always)]
fn fold(acc: (f64, f64), a: (f64, f64)) -> (f64, f64) { (acc.0 + a.0, acc.1 + 2.0 * acc.0 * a.0 + a.1) }

// ---------------- baseline: hash-map DP, AC-vector values ----------------
type V = ([f64; ACS], [f64; ACS]);
fn interp(sh: &Shape, p: &Params) -> V {
    let mut memo: FxHashMap<u64, V> = FxHashMap::default();
    let probs: Vec<[[f64; 3]; 2]> = (0..ACS)
        .map(|a| [outcome(p.to_hit, 10 + a as i32, p.crit_on, false), outcome(p.to_hit, 10 + a as i32, p.crit_on, true)])
        .collect();
    fn go(sh: &Shape, p: &Params, probs: &[[[f64; 3]; 2]], s: St, memo: &mut FxHashMap<u64, V>) -> V {
        let k = s.key();
        if let Some(v) = memo.get(&k) { return *v; }
        let v = match step(sh, s) {
            Step::Leaf => ([0.0; ACS], [0.0; ACS]),
            Step::Chance { adv, out } => {
                let kids: Vec<(f64, f64, V)> = out.iter().map(|(ts, n)| {
                    let d = ts.iter().fold((0.0, 0.0), |acc, &t| fold(acc, term(p, t)));
                    (d.0, d.1, go(sh, p, probs, *n, memo))
                }).collect();
                let mut e = [0.0; ACS]; let mut e2 = [0.0; ACS];
                for a in 0..ACS {
                    let pr = probs[a][adv as usize];
                    for o in 0..3 {
                        let (de, de2, ref c) = kids[o];
                        let ce = de + c.0[a];
                        let ce2 = de2 + 2.0 * de * c.0[a] + c.1[a];
                        e[a] += pr[o] * ce; e2[a] += pr[o] * ce2;
                    }
                }
                (e, e2)
            }
            Step::Decide { opts } => {
                let kids: Vec<(f64, f64, V)> = opts.iter().map(|(ts, n)| {
                    let d = ts.iter().fold((0.0, 0.0), |acc, &t| fold(acc, term(p, t)));
                    (d.0, d.1, go(sh, p, probs, *n, memo))
                }).collect();
                let mut e = [f64::NEG_INFINITY; ACS]; let mut e2 = [0.0; ACS];
                for a in 0..ACS {
                    for (de, de2, c) in &kids {
                        let ce = de + c.0[a];
                        if ce > e[a] { e[a] = ce; e2[a] = de2 + 2.0 * de * c.0[a] + c.1[a]; }
                    }
                }
                (e, e2)
            }
        };
        memo.insert(k, v);
        v
    }
    go(sh, p, &probs, root(sh), &mut memo)
}

// ---------------- tracer + tape ----------------
#[derive(Clone, Copy)]
struct Edge { to: u32, t0: u32, tn: u32 } // terms [t0, t0+tn) in tape.terms
#[derive(Clone, Copy)]
enum Node { Leaf, Chance { adv: bool, e: u32 }, Decide { e: u32, n: u32 } } // Chance uses 3 edges at e
struct Tape { nodes: Vec<Node>, edges: Vec<Edge>, terms: Vec<u8>, live_slots: usize, slot_of: Vec<u32>,
               label: Vec<(u8, u8, u8)> } // per node: (round, phase, slots_total) for Combat Script; empty after reduce()

fn trace(sh: &Shape) -> Tape {
    let mut ids: FxHashMap<u64, u32> = FxHashMap::default();
    let mut tape = Tape { nodes: vec![], edges: vec![], terms: vec![], live_slots: 0, slot_of: vec![], label: vec![] };
    fn go(sh: &Shape, s: St, ids: &mut FxHashMap<u64, u32>, tape: &mut Tape) -> u32 {
        let k = s.key();
        if let Some(&i) = ids.get(&k) { return i; }
        let node_edges: Vec<(Vec<u8>, u32)>;
        let kind = match step(sh, s) {
            Step::Leaf => { node_edges = vec![]; 0u8 }
            Step::Chance { adv, out } => {
                node_edges = out.into_iter().map(|(ts, n)| (ts, go(sh, n, ids, tape))).collect();
                1 + adv as u8
            }
            Step::Decide { opts } => {
                node_edges = opts.into_iter().map(|(ts, n)| (ts, go(sh, n, ids, tape))).collect();
                3
            }
        };
        let e0 = tape.edges.len() as u32;
        for (ts, to) in &node_edges {
            let t0 = tape.terms.len() as u32;
            tape.terms.extend_from_slice(ts);
            tape.edges.push(Edge { to: *to, t0, tn: ts.len() as u32 });
        }
        let node = match kind {
            0 => Node::Leaf,
            1 | 2 => Node::Chance { adv: kind == 2, e: e0 },
            _ => Node::Decide { e: e0, n: node_edges.len() as u32 },
        };
        let id = tape.nodes.len() as u32; // post-order: children already have smaller ids
        tape.nodes.push(node);
        tape.label.push((s.round, s.phase, s.slots.iter().sum()));
        ids.insert(k, id);
        id
    }
    go(sh, root(sh), &mut ids, &mut tape);
    regalloc(&mut tape);
    tape
}

/// Liveness-based slot reuse: a node's value is freed after its last parent is evaluated.
fn regalloc(t: &mut Tape) {
    let n = t.nodes.len();
    let mut last_use = vec![0u32; n];
    for (i, node) in t.nodes.iter().enumerate() {
        let (e, k) = match *node { Node::Leaf => (0, 0), Node::Chance { e, .. } => (e, 3), Node::Decide { e, n } => (e, n) };
        for j in 0..k { let c = t.edges[(e + j) as usize].to as usize; last_use[c] = last_use[c].max(i as u32); }
    }
    last_use[n - 1] = n as u32; // root lives to the end
    let mut free: Vec<u32> = vec![];
    let mut slot_of = vec![0u32; n];
    let mut next = 0u32;
    // frees scheduled at step i: nodes whose last_use == i
    let mut dies_at: Vec<Vec<u32>> = vec![vec![]; n + 1];
    for c in 0..n { dies_at[last_use[c] as usize].push(c as u32); }
    for i in 0..n {
        let s = free.pop().unwrap_or_else(|| { next += 1; next - 1 });
        slot_of[i] = s;
        for &c in &dies_at[i] { free.push(slot_of[c as usize]); }
    }
    t.slot_of = slot_of;
    t.live_slots = next as usize;
}

/// Evaluate the tape over a lane block: `kits` kits x 16 ACs. Returns root (E,E2) per lane.
fn eval_tape(t: &Tape, ps: &[Params]) -> (Vec<f64>, Vec<f64>) {
    let lanes = ps.len() * ACS;
    // per-lane outcome probs for adv=false/true
    let mut pr = vec![[0.0f64; 3]; 2 * lanes];
    for (k, p) in ps.iter().enumerate() {
        for a in 0..ACS {
            pr[k * ACS + a] = outcome(p.to_hit, 10 + a as i32, p.crit_on, false);
            pr[lanes + k * ACS + a] = outcome(p.to_hit, 10 + a as i32, p.crit_on, true);
        }
    }
    // SoA prob arrays
    let mut p_m = vec![0.0; 2 * lanes]; let mut p_h = vec![0.0; 2 * lanes]; let mut p_c = vec![0.0; 2 * lanes];
    for i in 0..2 * lanes { p_m[i] = pr[i][0]; p_h[i] = pr[i][1]; p_c[i] = pr[i][2]; }
    // per-edge folded damage moments per kit, expanded to lanes lazily: precompute per edge per kit
    let nk = ps.len();
    let mut ed_e = vec![0.0; t.edges.len() * nk];
    let mut ed_e2 = vec![0.0; t.edges.len() * nk];
    for (ei, ed) in t.edges.iter().enumerate() {
        for (k, p) in ps.iter().enumerate() {
            let mut acc = (0.0, 0.0);
            for &tm in &t.terms[ed.t0 as usize..(ed.t0 + ed.tn) as usize] { acc = fold(acc, term(p, tm)); }
            ed_e[ei * nk + k] = acc.0; ed_e2[ei * nk + k] = acc.1;
        }
    }
    let mut ve = vec![0.0f64; t.live_slots * lanes];
    let mut ve2 = vec![0.0f64; t.live_slots * lanes];
    let mut tmp_e = vec![0.0f64; lanes];
    let mut tmp_e2 = vec![0.0f64; lanes];
    for (i, node) in t.nodes.iter().enumerate() {
        match *node {
            Node::Leaf => { tmp_e.fill(0.0); tmp_e2.fill(0.0); }
            Node::Chance { adv, e } => {
                let off = if adv { lanes } else { 0 };
                tmp_e.fill(0.0); tmp_e2.fill(0.0);
                for o in 0..3 {
                    let ed = t.edges[(e + o) as usize];
                    let cs = t.slot_of[ed.to as usize] as usize * lanes;
                    let probs = match o { 0 => &p_m, 1 => &p_h, _ => &p_c };
                    for k in 0..nk {
                        let de = ed_e[(e + o) as usize * nk + k];
                        let de2 = ed_e2[(e + o) as usize * nk + k];
                        let base = k * ACS;
                        for a in 0..ACS {
                            let l = base + a;
                            let ce = de + ve[cs + l];
                            let ce2 = de2 + 2.0 * de * ve[cs + l] + ve2[cs + l];
                            let pp = probs[off + l];
                            tmp_e[l] += pp * ce; tmp_e2[l] += pp * ce2;
                        }
                    }
                }
            }
            Node::Decide { e, n } => {
                tmp_e.fill(f64::NEG_INFINITY); tmp_e2.fill(0.0);
                for j in 0..n {
                    let ed = t.edges[(e + j) as usize];
                    let cs = t.slot_of[ed.to as usize] as usize * lanes;
                    for k in 0..nk {
                        let de = ed_e[(e + j) as usize * nk + k];
                        let de2 = ed_e2[(e + j) as usize * nk + k];
                        let base = k * ACS;
                        for a in 0..ACS {
                            let l = base + a;
                            let ce = de + ve[cs + l];
                            let ce2 = de2 + 2.0 * de * ve[cs + l] + ve2[cs + l];
                            let better = ce > tmp_e[l];
                            tmp_e[l] = if better { ce } else { tmp_e[l] };
                            tmp_e2[l] = if better { ce2 } else { tmp_e2[l] };
                        }
                    }
                }
            }
        }
        let s = t.slot_of[i] as usize * lanes;
        ve[s..s + lanes].copy_from_slice(&tmp_e);
        ve2[s..s + lanes].copy_from_slice(&tmp_e2);
    }
    let r = t.slot_of[t.nodes.len() - 1] as usize * lanes;
    (ve[r..r + lanes].to_vec(), ve2[r..r + lanes].to_vec())
}

// ---------------- synthetic kits ----------------
fn dice(n: f64, s: f64) -> (f64, f64) { let e = n * (s + 1.0) / 2.0; let var = n * (s * s - 1.0) / 12.0; (e, var + e * e) }
fn plus(m: (f64, f64), c: f64) -> (f64, f64) { (m.0 + c, m.1 + 2.0 * c * m.0 + c * c) }
fn kits(n: usize, seed: u64) -> Vec<Params> {
    let mut x = seed;
    let mut rnd = |m: u64| { x ^= x << 13; x ^= x >> 7; x ^= x << 17; x % m };
    (0..n).map(|_| {
        let (dn, ds) = [(2.0, 6.0), (1.0, 12.0), (1.0, 10.0), (1.0, 8.0)][rnd(4) as usize];
        let m = 3.0 + rnd(3) as f64;
        let mut smite = [((0.0, 0.0), (0.0, 0.0)); 5];
        for l in 0..5 { smite[l] = (dice(2.0 + l as f64, 8.0), dice(2.0 * (2.0 + l as f64), 8.0)); }
        Params { to_hit: 5 + rnd(5) as i32, crit_on: 20 - rnd(2) as i32,
                 w_hit: plus(dice(dn, ds), m), w_crit: plus(dice(2.0 * dn, ds), m), smite }
    }).collect()
}

/// E-only evaluation of a priced (resource-free) tape for one kit; λ per level per lane.
fn eval_priced(t: &Tape, p: &Params, pr: &[[[f64; 3]; ACS]; 2], lam: &[[f64; ACS]; 5], ve: &mut Vec<[f64; ACS]>) -> [f64; ACS] {
    ve.resize(t.live_slots, [0.0; ACS]);
    let edge_val = |ed: &Edge, a: usize| -> f64 {
        let mut d = 0.0;
        for &tm in &t.terms[ed.t0 as usize..(ed.t0 + ed.tn) as usize] {
            d += if tm >= 12 { -lam[(tm - 12) as usize][a] } else { term(p, tm).0 };
        }
        d
    };
    for (i, node) in t.nodes.iter().enumerate() {
        let mut out = [0.0; ACS];
        match *node {
            Node::Leaf => {}
            Node::Chance { adv, e } => for o in 0..3 {
                let ed = &t.edges[(e + o) as usize];
                let c = ve[t.slot_of[ed.to as usize] as usize];
                for a in 0..ACS { out[a] += pr[adv as usize][a][o as usize] * (edge_val(ed, a) + c[a]); }
            },
            Node::Decide { e, n } => { out = [f64::NEG_INFINITY; ACS]; for j in 0..n {
                let ed = &t.edges[(e + j) as usize];
                let c = ve[t.slot_of[ed.to as usize] as usize];
                for a in 0..ACS { let v = edge_val(ed, a) + c[a]; if v > out[a] { out[a] = v; } }
            }}
        }
        ve[t.slot_of[i] as usize] = out;
    }
    ve[t.slot_of[t.nodes.len() - 1] as usize]
}

/// Lagrangian dual bound: min over λ ≥ 0 of V_λ + Σ λ_l S_l, by per-lane coordinate ternary search.
fn dual_bound(t: &Tape, sh: &Shape, p: &Params, cycles: usize, iters: usize) -> ([f64; ACS], usize) {
    let mut pr = [[[0.0; 3]; ACS]; 2];
    for a in 0..ACS { pr[0][a] = outcome(p.to_hit, 10 + a as i32, p.crit_on, false); pr[1][a] = outcome(p.to_hit, 10 + a as i32, p.crit_on, true); }
    let lv = sh.slots.len();
    let mut lam = [[0.0; ACS]; 5];
    // init: λ_l = half the smite's hit damage (a cheap guess)
    for l in 0..lv { lam[l] = [p.smite[l].0 .0 * 0.5; ACS]; }
    let mut ve = vec![];
    let mut evals = 0;
    let mut dual = |lam: &[[f64; ACS]; 5], ve: &mut Vec<[f64; ACS]>| -> [f64; ACS] {
        evals += 1;
        let mut v = eval_priced(t, p, &pr, lam, ve);
        for l in 0..lv { for a in 0..ACS { v[a] += lam[l][a] * sh.slots[l] as f64; } }
        v
    };
    for _ in 0..cycles { for l in 0..lv {
        let mut lo = [0.0; ACS]; let mut hi = [p.smite[l].1 .0; ACS];
        for _ in 0..iters {
            let mut l1 = lam; let mut l2 = lam;
            for a in 0..ACS { l1[l][a] = lo[a] + (hi[a] - lo[a]) / 3.0; l2[l][a] = hi[a] - (hi[a] - lo[a]) / 3.0; }
            let (d1, d2) = (dual(&l1, &mut ve), dual(&l2, &mut ve));
            for a in 0..ACS { if d1[a] <= d2[a] { hi[a] = l2[l][a]; } else { lo[a] = l1[l][a]; } }
        }
        for a in 0..ACS { lam[l][a] = (lo[a] + hi[a]) / 2.0; }
    }}
    let v = dual(&lam, &mut ve);
    (v, evals)
}

fn lagrange_experiment() {
    println!("\n#### Lagrangian (resource-priced) bound vs exact MDP");
    for (name, slots, horizon) in [("L8 slots 4/3, 3 rds", vec![4u8, 3], 3u8), ("L8 slots 4/3, 10 rds", vec![4, 3], 10),
                                   ("L20 slots 4/3/3/3/2, 3 rds", vec![4, 3, 3, 3, 2], 3), ("L20 slots 4/3/3/3/2, 10 rds", vec![4, 3, 3, 3, 2], 10),
                                   ("L20 slots 4/3/3/3/2, 12 rds (day-ish)", vec![4, 3, 3, 3, 2], 12)] {
        let exact_sh = Shape { attacks: 2, slots: slots.clone(), horizon, hew: true, vex: true, canon: false, priced: false };
        let pr_sh = Shape { priced: true, ..exact_sh.clone() };
        let te = trace(&exact_sh); let tp = trace(&pr_sh);
        let ks = kits(48, 0xC0FFEE);
        let (t_ex, ex) = best_of(2, || ks.iter().map(|p| eval_tape(&te, std::slice::from_ref(p)).0).collect::<Vec<_>>());
        for (cyc, it) in [(1usize, 6usize), (2, 10), (3, 16)] {
            let (t_du, du) = best_of(2, || ks.iter().map(|p| dual_bound(&tp, &pr_sh, p, cyc, it)).collect::<Vec<_>>());
            let mut gaps: Vec<f64> = vec![]; let mut viol = 0;
            for k in 0..ks.len() { for a in 0..ACS {
                let g = (du[k].0[a] - ex[k][a]) / ex[k][a];
                if g < -1e-9 { viol += 1; }
                gaps.push(g);
            }}
            gaps.sort_by(f64::total_cmp);
            // ranking agreement at AC 17: Kendall-ish — fraction of kit pairs ordered the same
            let a = 7; let mut agree = 0; let mut tot = 0;
            for i in 0..ks.len() { for j in i + 1..ks.len() {
                tot += 1; if (ex[i][a] < ex[j][a]) == (du[i].0[a] < du[j].0[a]) { agree += 1; }
            }}
            println!("  {name:<38} exact tape {:>4} nodes {:>8.1} µs/kit | dual tape {:>3} nodes, cyc {cyc} it {it:>2}: {:>7.1} µs/kit ({:>3} evals), gap median {:.3}% p95 {:.3}% max {:.3}%, bound violations {viol}, pair order agree {:.1}%",
                te.nodes.len(), t_ex / ks.len() as f64 * 1e6, tp.nodes.len(), t_du / ks.len() as f64 * 1e6, du[0].1,
                100.0 * gaps[gaps.len() / 2], 100.0 * gaps[gaps.len() * 95 / 100], 100.0 * gaps[gaps.len() - 1], 100.0 * agree as f64 / tot as f64);
        }
    }
}

fn canon_experiment() {
    println!("\n#### Resource canonicalisation (top-R slots) vs plain exact tape");
    for (slots, horizon) in [(vec![4u8, 3], 3u8), (vec![4, 3], 10), (vec![4, 3, 3, 3, 2], 3), (vec![4, 3, 3, 3, 2], 10), (vec![4, 3, 3, 3, 2], 12), (vec![4, 3, 3, 3, 2], 20)] {
        let a = Shape { attacks: 2, slots: slots.clone(), horizon, hew: true, vex: true, canon: false, priced: false };
        let b = Shape { canon: true, ..a.clone() };
        let (tta, ta) = best_of(2, || trace(&a)); let (ttb, tb) = best_of(2, || trace(&b));
        let ks = kits(64, 0xBEEF);
        let (t1, r1) = best_of(2, || ks.iter().map(|p| eval_tape(&ta, std::slice::from_ref(p))).collect::<Vec<_>>());
        let (t2, r2) = best_of(2, || ks.iter().map(|p| eval_tape(&tb, std::slice::from_ref(p))).collect::<Vec<_>>());
        let mut md = 0.0f64; let mut bits = true;
        for k in 0..ks.len() { for l in 0..ACS {
            md = md.max((r1[k].0[l] - r2[k].0[l]).abs() / r1[k].0[l]).max((r1[k].1[l] - r2[k].1[l]).abs() / r1[k].1[l]);
            bits &= r1[k].0[l].to_bits() == r2[k].0[l].to_bits();
        }}
        println!("  slots {:?} {:>2} rds: nodes {:>6} -> {:>6} ({:>4.1}x fewer), trace {:>6.2} -> {:>5.2} ms, eval {:>8.1} -> {:>7.1} µs/kit ({:>4.1}x), max rel diff {md:.1e}, bit-identical {bits}",
            slots, horizon, ta.nodes.len(), tb.nodes.len(), ta.nodes.len() as f64 / tb.nodes.len() as f64, tta * 1e3, ttb * 1e3,
            t1 / ks.len() as f64 * 1e6, t2 / ks.len() as f64 * 1e6, t1 / t2);
    }
}

fn thread_experiment() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    println!("\n#### Thread scaling (job queue, one kit per job, canonical tapes), M5 4P+6E");
    for (name, slots, horizon, nk) in [("L8 4/3, 3 rds", vec![4u8, 3], 3u8, 20000usize), ("L20 4/3/3/3/2, 12 rds", vec![4, 3, 3, 3, 2], 12, 600)] {
        let sh = Shape { attacks: 2, slots, horizon, hew: true, vex: true, canon: true, priced: false };
        let tape = trace(&sh);
        let ks = kits(nk, 7);
        let mut base = 0.0; let mut ref_bits: Vec<u64> = vec![];
        for nt in [1usize, 2, 4, 6, 8, 10] {
            let (t, out) = best_of(3, || {
                let next = AtomicUsize::new(0);
                let mut res = vec![0.0f64; nk * ACS];
                let slots: Vec<std::sync::Mutex<&mut [f64]>> = res.chunks_mut(ACS).map(std::sync::Mutex::new).collect();
                std::thread::scope(|sc| for _ in 0..nt { sc.spawn(|| loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    if i >= nk { break; }
                    let (e, _) = eval_tape(&tape, std::slice::from_ref(&ks[i]));
                    slots[i].lock().unwrap().copy_from_slice(&e);
                });});
                drop(slots);
                res
            });
            let bits: Vec<u64> = out.iter().map(|x| x.to_bits()).collect();
            if nt == 1 { base = t; ref_bits = bits.clone(); }
            println!("  {name:<22} {nt:>2} threads: {:>8.1} ms  {:>5.2} µs/kit  speedup {:>4.1}x  identical {}", t * 1e3, t / nk as f64 * 1e6, base / t, bits == ref_bits);
        }
    }
}

/// Independent oracle: Monte Carlo over the state machine (not the tape), policy = argmax of E from a
/// per-AC hash-map value table, damage terms sampled from a two-point law with the same E and E2.
fn mc_experiment() {
    println!("\n#### Monte Carlo oracle vs tape moments (1e6 trajectories each)");
    let sh = Shape { attacks: 2, slots: vec![4, 3, 3, 3, 2], horizon: 4, hew: true, vex: true, canon: true, priced: false };
    let tape = trace(&sh);
    for (ki, p) in kits(3, 99).iter().enumerate() {
        let (te, te2) = eval_tape(&tape, std::slice::from_ref(p));
        for &ac in &[13usize, 17, 21] {
            let a = ac - 10;
            // value table for this AC via the reference interpreter's recursion (E only)
            let mut memo: FxHashMap<u64, f64> = FxHashMap::default();
            fn val(sh: &Shape, p: &Params, ac: i32, s: St, m: &mut FxHashMap<u64, f64>) -> f64 {
                if let Some(v) = m.get(&s.key()) { return *v; }
                let d = |ts: &Vec<u8>| ts.iter().map(|&t| term(p, t).0).sum::<f64>();
                let v = match step(sh, s) {
                    Step::Leaf => 0.0,
                    Step::Chance { adv, out } => { let pr = outcome(p.to_hit, ac, p.crit_on, adv);
                        (0..3).map(|o| pr[o] * (d(&out[o].0) + val(sh, p, ac, out[o].1, m))).sum() }
                    Step::Decide { opts } => opts.iter().map(|(ts, n)| d(ts) + val(sh, p, ac, *n, m)).fold(f64::NEG_INFINITY, f64::max),
                };
                m.insert(s.key(), v); v
            }
            let mut x: u64 = 0x1234_5678 ^ (ki as u64 * 7919 + ac as u64);
            let mut u = || { x ^= x << 13; x ^= x >> 7; x ^= x << 17; (x >> 11) as f64 / (1u64 << 53) as f64 };
            let n = 1_000_000; let (mut s1, mut s2) = (0.0, 0.0);
            for _ in 0..n {
                let mut st = root(&sh); let mut tot = 0.0;
                loop {
                    let mut add = |ts: &Vec<u8>, u: &mut dyn FnMut() -> f64| for &t in ts { let (e, e2) = term(p, t); let sd = (e2 - e * e).max(0.0).sqrt(); tot += if u() < 0.5 { e - sd } else { e + sd }; };
                    match step(&sh, st) {
                        Step::Leaf => break,
                        Step::Chance { adv, out } => { let pr = outcome(p.to_hit, ac as i32, p.crit_on, adv); let r = u();
                            let o = if r < pr[0] { 0 } else if r < pr[0] + pr[1] { 1 } else { 2 }; add(&out[o].0, &mut u); st = out[o].1; }
                        Step::Decide { opts } => { let mut best = 0; let mut bv = f64::NEG_INFINITY;
                            for (j, (ts, nx)) in opts.iter().enumerate() { let v = ts.iter().map(|&t| term(p, t).0).sum::<f64>() + val(&sh, p, ac as i32, *nx, &mut memo); if v > bv { bv = v; best = j; } }
                            add(&opts[best].0, &mut u); st = opts[best].1; }
                    }
                }
                s1 += tot; s2 += tot * tot;
            }
            let (m1, m2) = (s1 / n as f64, s2 / n as f64);
            let sd = (m2 - m1 * m1).sqrt(); let se = sd / (n as f64).sqrt();
            let tsd = (te2[a] - te[a] * te[a]).sqrt();
            println!("  kit {ki} AC {ac}: tape E {:>8.3} SD {:>7.3} | MC E {:>8.3} ± {:.3} (2σ) SD {:>7.3} | z(E) = {:+.2}, SD rel diff {:+.2}%",
                te[a], tsd, m1, 2.0 * se, sd, (m1 - te[a]) / se, 100.0 * (sd - tsd) / tsd);
        }
    }
}

/// BDD-style reduction (Bryant's two rules, applied by hash-consing in post-order):
/// (1) merge structurally identical nodes (same kind, same edges = same terms + same reduced child);
/// (2) drop pass-through nodes (a Decide with one option and no terms).
fn reduce(t: &Tape) -> Tape {
    let mut out = Tape { nodes: vec![], edges: vec![], terms: vec![], live_slots: 0, slot_of: vec![], label: vec![] };
    let mut rep = vec![0u32; t.nodes.len()];
    let mut uniq: FxHashMap<Vec<u32>, u32> = FxHashMap::default();
    for (i, node) in t.nodes.iter().enumerate() {
        let (tag, e, k) = match *node { Node::Leaf => (0u32, 0, 0), Node::Chance { adv, e } => (1 + adv as u32, e, 3), Node::Decide { e, n } => (3, e, n) };
        if tag == 3 && k == 1 && t.edges[e as usize].tn == 0 { rep[i] = rep[t.edges[e as usize].to as usize]; continue; }
        let mut sig = vec![tag];
        for j in 0..k {
            let ed = t.edges[(e + j) as usize];
            sig.push(rep[ed.to as usize]); sig.push(ed.tn);
            sig.extend(t.terms[ed.t0 as usize..(ed.t0 + ed.tn) as usize].iter().map(|&x| x as u32));
        }
        if let Some(&id) = uniq.get(&sig) { rep[i] = id; continue; }
        let e0 = out.edges.len() as u32;
        for j in 0..k {
            let ed = t.edges[(e + j) as usize];
            let t0 = out.terms.len() as u32;
            out.terms.extend_from_slice(&t.terms[ed.t0 as usize..(ed.t0 + ed.tn) as usize]);
            out.edges.push(Edge { to: rep[ed.to as usize], t0, tn: ed.tn });
        }
        out.nodes.push(match tag { 0 => Node::Leaf, 1 | 2 => Node::Chance { adv: tag == 2, e: e0 }, _ => Node::Decide { e: e0, n: k } });
        let id = out.nodes.len() as u32 - 1;
        uniq.insert(sig, id); rep[i] = id;
    }
    regalloc(&mut out);
    out
}

fn reduce_experiment() {
    println!("\n#### BDD-style reduction (hash-consing + pass-through removal) on top of canonicalisation");
    for (slots, hz, hew, vex) in [(vec![], 3u8, true, true), (vec![4u8, 3], 3, true, true), (vec![4, 3, 3, 3, 2], 10, true, true),
                                  (vec![4, 3, 3, 3, 2], 12, true, true), (vec![4, 3, 3, 3, 2], 12, false, false), (vec![4, 3, 3, 3, 2], 20, true, true)] {
        let sh = Shape { attacks: 2, slots: slots.clone(), horizon: hz, hew, vex, canon: true, priced: false };
        let t = trace(&sh);
        let (tr, r) = best_of(2, || reduce(&t));
        let ks = kits(64, 0xABCD);
        let (t1, a) = best_of(2, || ks.iter().map(|p| eval_tape(&t, std::slice::from_ref(p))).collect::<Vec<_>>());
        let (t2, b) = best_of(2, || ks.iter().map(|p| eval_tape(&r, std::slice::from_ref(p))).collect::<Vec<_>>());
        let bits = a.iter().zip(&b).all(|(x, y)| x.0.iter().zip(&y.0).all(|(u, v)| u.to_bits() == v.to_bits()) && x.1.iter().zip(&y.1).all(|(u, v)| u.to_bits() == v.to_bits()));
        println!("  slots {:<16} {:>2} rds hew={hew:<5} vex={vex:<5}: nodes {:>6} -> {:>6} ({:.2}x), reduce {:>6.2} ms, eval {:>7.1} -> {:>7.1} µs/kit, bit-identical {bits}",
            format!("{:?}", slots), hz, t.nodes.len(), r.nodes.len(), t.nodes.len() as f64 / r.nodes.len() as f64, tr * 1e3,
            t1 / 64.0 * 1e6, t2 / 64.0 * 1e6);
    }
}

/// Backward pass that also records the argmax choice per Decide node per lane (full arrays, no slot reuse).
fn backward_with_policy(t: &Tape, p: &Params) -> (Vec<[f64; ACS]>, Vec<[u8; ACS]>, [[[f64; 3]; ACS]; 2]) {
    let mut pr = [[[0.0; 3]; ACS]; 2];
    for a in 0..ACS { pr[0][a] = outcome(p.to_hit, 10 + a as i32, p.crit_on, false); pr[1][a] = outcome(p.to_hit, 10 + a as i32, p.crit_on, true); }
    let dmg = |ed: &Edge| t.terms[ed.t0 as usize..(ed.t0 + ed.tn) as usize].iter().fold((0.0, 0.0), |acc, &x| fold(acc, term(p, x))).0;
    let mut v = vec![[0.0; ACS]; t.nodes.len()];
    let mut ch = vec![[0u8; ACS]; t.nodes.len()];
    for (i, node) in t.nodes.iter().enumerate() {
        match *node {
            Node::Leaf => {}
            Node::Chance { adv, e } => for o in 0..3 { let ed = &t.edges[(e + o) as usize]; let d = dmg(ed);
                for a in 0..ACS { v[i][a] += pr[adv as usize][a][o as usize] * (d + v[ed.to as usize][a]); } },
            Node::Decide { e, n } => { v[i] = [f64::NEG_INFINITY; ACS];
                for j in 0..n { let ed = &t.edges[(e + j) as usize]; let d = dmg(ed);
                    for a in 0..ACS { let c = d + v[ed.to as usize][a]; if c > v[i][a] { v[i][a] = c; ch[i][a] = j as u8; } } } }
        }
    }
    (v, ch, pr)
}

/// Forward (occupancy) pass under a fixed per-lane policy: reach probability of every node, and the expected
/// number of times each damage term is applied. Darwiche's "evaluate up, differentiate down", for an MDP.
fn forward(t: &Tape, ch: &[[u8; ACS]], pr: &[[[f64; 3]; ACS]; 2], n_terms: usize) -> (Vec<[f64; ACS]>, Vec<[f64; ACS]>) {
    let n = t.nodes.len();
    let mut reach = vec![[0.0; ACS]; n];
    let mut uses = vec![[0.0; ACS]; n_terms];
    reach[n - 1] = [1.0; ACS];
    for i in (0..n).rev() {
        let r = reach[i];
        match t.nodes[i] {
            Node::Leaf => {}
            Node::Chance { adv, e } => for o in 0..3 { let ed = t.edges[(e + o) as usize];
                for a in 0..ACS { let m = r[a] * pr[adv as usize][a][o as usize]; reach[ed.to as usize][a] += m;
                    for &x in &t.terms[ed.t0 as usize..(ed.t0 + ed.tn) as usize] { uses[x as usize][a] += m; } } },
            Node::Decide { e, .. } => for a in 0..ACS { let ed = t.edges[(e + ch[i][a] as u32) as usize];
                reach[ed.to as usize][a] += r[a];
                for &x in &t.terms[ed.t0 as usize..(ed.t0 + ed.tn) as usize] { uses[x as usize][a] += r[a]; } },
        }
    }
    (reach, uses)
}

fn occupancy_experiment() {
    println!("\n#### Forward (occupancy) pass: attribution, exact sensitivities, Combat Script fidelity");
    let sh = Shape { attacks: 2, slots: vec![4, 3, 3, 3, 2], horizon: 12, hew: true, vex: true, canon: true, priced: false };
    let t = trace(&sh);
    let nt = 2 + 2 * sh.slots.len();
    for (ki, p) in kits(3, 2024).iter().enumerate() {
        let (tb, (v, ch, pr)) = best_of(3, || backward_with_policy(&t, p));
        let (tf, (_reach, uses)) = best_of(3, || forward(&t, &ch, &pr, nt));
        let root = v[t.nodes.len() - 1];
        // (a) linearity: Σ uses × E[term] == E
        let mut maxrel = 0.0f64;
        for a in 0..ACS { let s: f64 = (0..nt).map(|x| uses[x][a] * term(p, x as u8).0).sum(); maxrel = maxrel.max(((s - root[a]) / root[a]).abs()); }
        // (b) exact sensitivity: dE/d(mean weapon damage on hit) == expected weapon hits; check by finite difference
        let eps = 1e-4;
        let mut q = *p; q.w_hit = plus(p.w_hit, eps);
        let v2 = backward_with_policy(&t, &q).0[t.nodes.len() - 1];
        let fd = (v2[7] - root[7]) / eps;
        // (c) +1 to-hit == shift by one AC lane, bit-for-bit
        let mut q1 = *p; q1.to_hit += 1;
        let v3 = backward_with_policy(&t, &q1).0[t.nodes.len() - 1];
        let shift_ok = (1..ACS).all(|a| v3[a].to_bits() == root[a - 1].to_bits());
        let slots_used: Vec<String> = (0..sh.slots.len()).map(|l| format!("{:.2}", uses[2 + 2 * l][7] + uses[3 + 2 * l][7])).collect();
        println!("  kit {ki}: E@AC17 {:.2} | backward {:.0} µs, forward {:.0} µs (all {} nodes, 16 lanes) | Σ uses·E vs E: max rel {maxrel:.1e} | dE/dw_hit: occupancy {:.4} vs finite-diff {:.4} | +1 to-hit == AC lane shift (bit-exact): {shift_ok} | expected slots used L1..L5 @AC17: [{}]",
            root[7], tb * 1e6, tf * 1e6, t.nodes.len(), uses[0][7], fd, slots_used.join(", "));
        // (d) Combat Script fidelity: value of simple human rules, evaluated exactly by forward pass (policy evaluation)
        let rule = |name: &str, f: &dyn Fn(u8, u8, u8, &[u8]) -> usize| {
            // f(round, phase, slots_total, option_levels(0=no smite)) -> option index
            let mut chr = ch.clone();
            for (i, node) in t.nodes.iter().enumerate() { if let Node::Decide { e, n } = *node {
                let (rd, ph, st) = t.label[i];
                let lv: Vec<u8> = (0..n).map(|j| { let ed = t.edges[(e + j) as usize];
                    t.terms[ed.t0 as usize..(ed.t0 + ed.tn) as usize].iter().find(|&&x| (2..12).contains(&x)).map(|&x| (x - 2) / 2 + 1).unwrap_or(0) }).collect();
                let pick = if ph == 3 { (n - 1) as usize } else { f(rd, ph, st, &lv) };
                chr[i] = [pick as u8; ACS];
            }}
            let (_, u) = forward(&t, &chr, &pr, nt);
            let val: Vec<f64> = (0..ACS).map(|a| (0..nt).map(|x| u[x][a] * term(p, x as u8).0).sum()).collect();
            // fidelity: reach-weighted agreement with the optimal choice at AC 17
            let (reach_opt, _) = forward(&t, &ch, &pr, nt);
            let (mut agree, mut tot) = (0.0, 0.0);
            for (i, node) in t.nodes.iter().enumerate() { if let Node::Decide { n, .. } = *node { if n > 1 {
                tot += reach_opt[i][7]; if chr[i][7] == ch[i][7] { agree += reach_opt[i][7]; } } } }
            println!("      rule {name:<44} loses {:>5.2}% @AC13, {:>5.2}% @AC17, {:>5.2}% @AC21; agrees with optimal on {:>5.1}% of decisions",
                100.0 * (1.0 - val[3] / root[3]), 100.0 * (1.0 - val[7] / root[7]), 100.0 * (1.0 - val[11] / root[11]), 100.0 * agree / tot);
        };
        let hi = |lv: &[u8]| (0..lv.len()).max_by_key(|&j| lv[j]).unwrap();
        let lo = |lv: &[u8]| (1..lv.len()).min_by_key(|&j| lv[j]).unwrap_or(0);
        rule("smite crits only, highest slot", &|_, ph, _, lv| if ph == 2 { hi(lv) } else { 0 });
        rule("smite every hit, highest slot", &|_, _, _, lv| hi(lv));
        rule("crit: highest; hit: lowest", &|_, ph, _, lv| if ph == 2 { hi(lv) } else { lo(lv) });
        // NB: labels carry the *canonical* slot total (clamped to turns left), so "slots > turns left" is never true.
        rule("crit: highest; hit: lowest if slots >= turns left", &|rd, ph, st, lv| if ph == 2 { hi(lv) } else if st as i32 >= (12 - rd as i32) { lo(lv) } else { 0 });
        rule("crit: highest; hit: highest if slots >= turns left", &|rd, ph, st, lv| if ph == 2 || st as i32 >= (12 - rd as i32) { hi(lv) } else { 0 });
    }
}

/// Metamorphic relations: exact monotonicity properties that need no oracle.
fn meta_experiment() {
    println!("\n#### Metamorphic relations (no oracle needed), 200 random kits x several shapes");
    let ev = |sh: &Shape, p: &Params| eval_tape(&trace(sh), std::slice::from_ref(p)).0;
    let mut fails = [0usize; 6]; let mut checks = [0usize; 6];
    let names = ["AC+1 never raises E", "+1 slot (any level) never lowers E", "+1 round never lowers E",
                 "more smite damage never lowers E", "enabling hew never lowers E", "+1 to-hit never lowers E"];
    for (si, (slots, hz)) in [(vec![2u8, 1], 3u8), (vec![4, 3], 4), (vec![2, 2, 1], 5)].into_iter().enumerate() {
        let base = Shape { attacks: 2, slots: slots.clone(), horizon: hz, hew: true, vex: true, canon: true, priced: false };
        for (ki, p) in kits(200, 77 + si as u64).iter().enumerate() {
            let e = ev(&base, p);
            for a in 0..ACS - 1 { checks[0] += 1; if e[a + 1] > e[a] { fails[0] += 1; } }
            let l = ki % slots.len(); let mut s2 = base.clone(); s2.slots[l] += 1;
            let e2 = ev(&s2, p); for a in 0..ACS { checks[1] += 1; if e2[a] < e[a] { fails[1] += 1; } }
            let s3 = Shape { horizon: hz + 1, ..base.clone() }; let e3 = ev(&s3, p);
            for a in 0..ACS { checks[2] += 1; if e3[a] < e[a] { fails[2] += 1; } }
            let mut q = *p; for x in q.smite.iter_mut() { x.0 = plus(x.0, 1.0); x.1 = plus(x.1, 1.0); }
            let e4 = ev(&base, &q); for a in 0..ACS { checks[3] += 1; if e4[a] < e[a] { fails[3] += 1; } }
            let s5 = Shape { hew: false, ..base.clone() }; let e5 = ev(&s5, p);
            for a in 0..ACS { checks[4] += 1; if e[a] < e5[a] { fails[4] += 1; } }
            let mut q6 = *p; q6.to_hit += 1; let e6 = ev(&base, &q6);
            for a in 0..ACS { checks[5] += 1; if e6[a] < e[a] { fails[5] += 1; } }
        }
    }
    for i in 0..6 { println!("  {:<38} {:>6} checks, {} violations", names[i], checks[i], fails[i]); }
}

fn best_of<T>(reps: usize, mut f: impl FnMut() -> T) -> (f64, T) {
    let mut best = f64::INFINITY; let mut out = None;
    for _ in 0..reps { let t = Instant::now(); let r = f(); best = best.min(t.elapsed().as_secs_f64()); out = Some(r); }
    (best, out.unwrap())
}

fn main() {
    let shapes = [
        ("L5 GWM-ish: 2 atk, hew, vex, no slots, 3 rds", Shape { attacks: 2, slots: vec![], horizon: 3, hew: true, vex: true, canon: false, priced: false }),
        ("L8 paladin-ish: 2 atk, slots 4/3, hew, vex, 3 rds", Shape { attacks: 2, slots: vec![4, 3], horizon: 3, hew: true, vex: true, canon: false, priced: false }),
        ("L8 paladin-ish, 10 rds", Shape { attacks: 2, slots: vec![4, 3], horizon: 10, hew: true, vex: true, canon: false, priced: false }),
        ("L20 paladin-ish: slots 4/3/3/3/2, 3 rds", Shape { attacks: 2, slots: vec![4, 3, 3, 3, 2], horizon: 3, hew: true, vex: true, canon: false, priced: false }),
        ("L20 paladin-ish, 10 rds", Shape { attacks: 2, slots: vec![4, 3, 3, 3, 2], horizon: 10, hew: true, vex: true, canon: false, priced: false }),
    ];
    if std::env::args().any(|a| a == "meta") { meta_experiment(); return; }
    if std::env::args().any(|a| a == "reduce") { reduce_experiment(); return; }
    if std::env::args().any(|a| a == "occ") { occupancy_experiment(); return; }
    if std::env::args().any(|a| a == "mc") { mc_experiment(); return; }
    if std::env::args().any(|a| a == "hash") {
        let mut h: u64 = 0xcbf29ce484222325;
        for (slots, hz) in [(vec![4u8, 3], 3u8), (vec![4, 3, 3, 3, 2], 10), (vec![4, 3, 3, 3, 2], 12)] {
            for canon in [false, true] {
                let sh = Shape { attacks: 2, slots: slots.clone(), horizon: hz, hew: true, vex: true, canon, priced: false };
                let t = trace(&sh);
                for p in kits(64, 42) { let (e, e2) = eval_tape(&t, &[p]); for x in e.iter().chain(&e2) { h = (h ^ x.to_bits()).wrapping_mul(0x100000001b3); } }
            }
        }
        println!("result-bits fnv64 = {h:016x}"); return;
    }
    if std::env::args().any(|a| a == "threads") { thread_experiment(); return; }
    if std::env::args().any(|a| a == "canon") { canon_experiment(); return; }
    if std::env::args().any(|a| a == "lagrange") { lagrange_experiment(); return; }
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    println!("threads available: {threads}");
    for (name, sh) in shapes.iter() {
        let (tt, tape) = best_of(3, || trace(sh));
        let bytes = tape.nodes.len() * std::mem::size_of::<Node>() + tape.edges.len() * 12 + tape.terms.len() + tape.slot_of.len() * 4;
        println!("\n== {name}\n  tape: {} nodes, {} edges, {} live slots ({:.1}% of nodes), {:.1} KB, trace {:.2} ms",
            tape.nodes.len(), tape.edges.len(), tape.live_slots, 100.0 * tape.live_slots as f64 / tape.nodes.len() as f64,
            bytes as f64 / 1024.0, tt * 1e3);
        let nk = 256;
        let ks = kits(nk, 0x9E3779B97F4A7C15);
        // baseline: per-kit hash-map DP
        let reps_i = 3;
        let sample = 32.min(nk);
        let (ti, vi) = best_of(reps_i, || ks[..sample].iter().map(|p| interp(sh, p)).collect::<Vec<_>>());
        let per_kit_interp = ti / sample as f64;
        // tape, varying block size
        let mut maxdiff = 0.0f64; let mut bitexact = true;
        for &b in &[1usize, 16, 64, 256] {
            let (tb, (e, e2)) = best_of(3, || {
                let mut e = vec![]; let mut e2 = vec![];
                for chunk in ks.chunks(b) { let (a, c) = eval_tape(&tape, chunk); e.extend(a); e2.extend(c); }
                (e, e2)
            });
            for k in 0..sample { for a in 0..ACS {
                let (x, y) = (vi[k].0[a], e[k * ACS + a]);
                let (x2, y2) = (vi[k].1[a], e2[k * ACS + a]);
                maxdiff = maxdiff.max((x - y).abs()).max((x2 - y2).abs() / x2.abs().max(1.0));
                if x.to_bits() != y.to_bits() || x2.to_bits() != y2.to_bits() { bitexact = false; }
            }}
            let per_kit = tb / nk as f64;
            println!("  block {:>3} kits: {:>8.2} µs/kit (16 ACs)  speedup vs hashmap DP {:>5.1}x", b, per_kit * 1e6, per_kit_interp / per_kit);
        }
        println!("  hashmap DP baseline: {:>8.2} µs/kit (16 ACs)", per_kit_interp * 1e6);
        println!("  tape == hashmap DP: bit-exact={bitexact}, max diff {maxdiff:.2e}");
        let k0 = &vi[0];
        println!("  sample kit0: E@AC10 {:.3}  E@AC17 {:.3}  E@AC25 {:.3}  SD@AC17 {:.3}", k0.0[0], k0.0[7], k0.0[15], (k0.1[7] - k0.0[7] * k0.0[7]).sqrt());
        // thread scaling + determinism, block 64
        let b = 64;
        let run = |nt: usize| -> (f64, Vec<f64>) {
            best_of(3, || {
                let chunks: Vec<&[Params]> = ks.chunks(b).collect();
                let mut out: Vec<Vec<f64>> = vec![vec![]; chunks.len()];
                std::thread::scope(|s| {
                    let per = chunks.len().div_ceil(nt);
                    for (ci, outs) in out.chunks_mut(per).enumerate() {
                        let chunks = &chunks; let tape = &tape;
                        s.spawn(move || for (j, o) in outs.iter_mut().enumerate() { *o = eval_tape(tape, chunks[ci * per + j]).0; });
                    }
                });
                out.concat()
            })
        };
        let (t1, r1) = run(1);
        let (tn, rn) = run(threads.min(4));
        let same = r1.iter().zip(&rn).all(|(a, b)| a.to_bits() == b.to_bits());
        println!("  threads 1 vs {}: {:.2} ms vs {:.2} ms ({:.1}x), identical bits={same}", threads.min(4), t1 * 1e3, tn * 1e3, t1 / tn);
    }
}
