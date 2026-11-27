# 03 — Probability and damage mathematics engine

> Research note for MinMax5e. Scope: how to compute exact DPR distributions, variance and reliability, and AC/save sensitivity for the 2024-rules MVP (Fighter, Rogue, Barbarian, Ranger, Paladin, Monk; single-target sustained and nova DPR).
> All numbers below were computed by the author of this note with a small Python/NumPy prototype (dense PMF arrays + direct convolution). The scripts were kept in a scratch directory outside the repo and are not part of the codebase; the algorithms are described in full below so they can be re-implemented.

## Question

What maths should the engine use to turn a build into (a) hit/crit/save probabilities, (b) an exact per-round and per-combat **damage distribution**, (c) reliability metrics that tell Build A ("85% hit, Graze floor") apart from Build B ("40% hit, swingy crits") even when both average 45 DPR, and (d) AC and save sensitivity curves against a defensible **encounter baseline**? And how do we keep it fast enough for a live dashboard and a build-space search?

## TL;DR

1. **Closed forms are enough for every d20 event.** With `t` = natural roll needed (clamped to `[2, critOn]`), `k` = d20s kept-highest (1 normal, 2 advantage, 3 Elven Accuracy), `P(hit) = 1 − ((t−1)/20)^k` and `P(crit) = 1 − ((critOn−1)/20)^k`; disadvantage is `((21−t)/20)^2`. Saves have **no** natural-1/20 rule in 2024, so `P(fail) = clamp((DC − bonus − 1)/20, 0, 1)` and Magic Resistance squares it.
2. **Use exact discrete PMFs, not Monte Carlo, as the source of truth.** Damage supports are tiny (a level-5 Fighter round has support 8–90; a 10-round fight about 900 values). Direct convolution of float64 arrays computes the worked example's full round PMF in **≈0.04 ms** and a 10-round fight in **≈0.07 ms**. Monte Carlo needs about 10⁵ samples (≈13 ms) to reach ±0.03 DPR, and it is noisy in the tails. FFT gains nothing at these sizes and produces tiny negative "probabilities" (we measured 255 negative entries of order −3×10⁻¹⁸).
3. **Model per-turn state with a small Markov chain / DP over attack slots.** States such as "crit seen yet" (Hew), "Sneak Attack used", "target prone" (Topple), "advantage on next attack" (Vex) and "slots left" (Smite) each hold an *unnormalised PMF*. This is exact and cheap until the state includes multiple independent targets, enemy decisions or long histories (see §3.4).
4. **Reliability should be reported as a kill-time distribution, with a per-round floor alongside.** In the spec's example both builds average 45/round, yet against a 135-HP target Build A's expected time-to-kill is **3.47 rounds** and Build B's is **3.95 rounds**, because B's spikes are wasted as overkill. SD (8.4 vs 42.2) and CV (0.19 vs 0.94) show the spread, but only TTK and P(kill within K) show what it *costs*. Variance is not uniformly bad: against a 60-HP target in **one** round B wins (34% vs 1.5%).
5. **Default encounter baseline: target CR = character level, with AC from Tom Dunn's baseline table (≈ 13 + CR/3).** Our own fit to the 330 stat blocks in SRD 5.2.1 (the 2024-era monsters) gives **AC ≈ 12.9 + 0.37·CR**, which agrees within half a point. Saves should use **per-ability** curves, not one number: Int and Cha are weak, and Str and Con are strong.
6. **Worked example.** A level-5 2024 Fighter (Str 18, greatsword, GWF + GWM + Graze + Hew) against AC 16 deals **mean 22.90, SD 10.26 per round**, with a hard floor of 8 (both attacks graze, 16%). The median is 20, the 90th percentile 34 and the maximum 90.

---

## 1. Closed-form d20 mathematics

### 1.1 Rules that fix the formulas (SRD 5.2.1)

| Rule | Text / effect | Source |
|---|---|---|
| Natural 20 / 1 on **attack rolls** | Nat 20 hits regardless of AC and is a Critical Hit; nat 1 misses regardless | SRD 5.2.1 p.7 "Rolling 20 or 1"; glossary "Critical Hit" p.179 — [SRD 5.2.1 PDF](https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf) |
| Saves | The 20/1 rule is stated only for attack rolls, so saves have no automatic success or failure | same, p.7 |
| Advantage does not stack | "you still roll only two d20s"; any Adv plus any Disadv gives a straight roll | p.8 "They Don't Stack" |
| Critical damage | "Roll the attack's damage dice twice … add any relevant modifiers as normal"; extra dice such as Sneak Attack are also doubled | p.16 "Critical Hits" |
| Champion Improved Critical | crit on 19–20 | p.49 |
| Resistance | Halved and rounded down, applied **after** bonuses and before Vulnerability | p.17 "Order of Application" |

### 1.2 Attack roll

Let `B` = attack bonus, `AC` = target AC, `c` = lowest natural roll that crits (20, or 19 for Champion), `k` = number of d20s for "keep highest" (1 normal, 2 advantage, 3 Elven Accuracy).

```
t      = clamp(AC − B, 2, c)       # natural roll needed; nat-1 always misses; crit-range rolls always hit (see Open Q1)
P_≥(x) = 1 − ((x−1)/20)^k          # keep-highest of k d20s
P_crit = P_≥(c)
P_hit  = P_≥(t)                    # includes crits
P_miss = 1 − P_hit
disadvantage: P_≥(x) = ((21−x)/20)^2
```

At `+7` against AC 16 (t = 9):

| Mode | P(crit) | P(non-crit hit) | P(miss) | P(crit) at crit 19–20 |
|---|---|---|---|---|
| Normal | 0.0500 | 0.5500 | 0.4000 | 0.1000 |
| Advantage | 0.0975 | 0.7425 | 0.1600 | 0.1900 |
| Elven Accuracy (3 dice, a 2014 P1 feat from Xanathar's Guide; not in the 2024 PHB — [aidedd 2024 feat index has no entry](https://www.aidedd.org/feat/elven-accuracy)) | 0.1426 | 0.7934 | 0.0640 | 0.2710 |
| Disadvantage | 0.0025 | 0.3575 | 0.6400 | 0.0100 |

### 1.3 Expected damage per attack

With hit damage `H` (dice + flat), crit damage `C` (dice doubled + flat) and miss damage `G` (Graze: "damage … equal to the ability modifier used", SRD p.90):

```
E[attack] = (P_hit − P_crit)·E[H] + P_crit·E[C] + P_miss·G
```

Flat riders such as GWM's Heavy Weapon Mastery (+PB on a hit with a Heavy weapon as part of the Attack action — [GWM 2024 text, aidedd transcription](https://www.aidedd.org/feat/great-weapon-master)) belong to the flat part and are **not** doubled on a crit.

### 1.4 Once-per-turn riders (Sneak Attack, Smite, Savage Attacker)

Sneak Attack: "Once per turn, you can deal an extra 1d6 … to one creature you hit" (SRD p.61). For `n` attacks with per-attack miss and crit probabilities `m_i` and `c_i`:

```
P(rider lands)          = 1 − Π m_i
P(rider lands on crit)  = Σ_i (Π_{j<i} m_j)·c_i            # greedy "use on first hit"
E[rider] = E[R]·P(rider lands) + E[R]·P(rider lands on crit)   # crit doubles rider dice
```

**Should the rider wait for a crit?** Holding a rider on a non-crit hit is worth `(P_hit−P_crit)·E[R] + P_crit·2E[R]` from the next attack. At +7 vs AC16 that is `0.55·E[R] + 0.10·E[R] = 0.65·E[R] < E[R]`, so the greedy choice is correct whenever a single later attack is all you are waiting for. Our rogue example (L5, Dex 18, shortsword + scimitar with Nick, SA 3d6, ally adjacent) gives `E[SA] = 10.5·0.84 + 10.5·0.07 = 9.555`, and the whole turn averages **16.51**, or **17.11** with Vex on the first attack.

**Smite with a limited slot budget is a real sequential decision.** 2024 Divine Smite is a Bonus Action spell taken "immediately after hitting" (SRD p.125), so it is limited to one per turn and the decision is made after seeing whether the hit crit. We solved the MDP exactly. Setup: Paladin L5, Str 18, GWF + GWM + Graze greatsword, 2 attacks, AC 16, 3 rounds, 2d8 per L1 slot.

| Slots for the fight | Greedy (smite first hit) | Crit-only | DP-optimal |
|---|---|---|---|
| 1 | 75.71 | 70.77 | **76.39** |
| 2 | 84.79 | 71.25 | **84.85** |
| ≥3 | 90.57 | 71.27 | 90.57 |

Crit-fishing loses 5–19 damage per fight at a 5% crit rate. The DP's advantage over greedy is under 1 point. With 1 slot, the DP holds the slot on a non-crit hit in rounds 1–2 and spends it freely in round 3. Expanded crit ranges and advantage change the balance, so the engine should solve the DP rather than hard-code the heuristic.

### 1.5 Saving throws

```
P(fail) = clamp((DC − SB − 1)/20, 0, 1)           # no nat-1/20 for saves in 2024
Magic Resistance (Adv on saves vs magic): P(fail)^2
"save for half": E = P(fail)·E[D] + (1−P(fail))·E[floor(D/2)]   # floor per instance, so use the PMF transform, not E[D]/2
```

### 1.6 2014 power-attack threshold (P1 sandbox)

For −5/+10 (2014 GWM/SS), ignoring crits, take the penalty iff `(p−0.25)(D+10) > p·D`, i.e. **`p > 0.25 + D/40`**, where `p` is the base hit chance and `D` the average hit damage. Example: `D = 11` (2d6+4) needs `p > 0.525`. The exact rule including crits uses §1.2–1.3 directly. This replaces the community rule of thumb with something that can be checked against a formula.

---

## 2. Full damage distributions

### 2.1 Representation and operations

A **damage PMF** is a dense `Float64Array` `p[0..N]` (plus an integer offset if negative values are ever needed). D&D damage is a non-negative integer, so dense arrays are natural.

| Operation | Implementation | Cost |
|---|---|---|
| Single die `dS` | uniform on 1..S | O(S) |
| Great Weapon Fighting (2024: treat 1–2 as 3, SRD p.88) | move the mass of faces 1 and 2 to 3 | O(S) |
| `NdS + k` | N-fold convolution, then shift by k | O(N²S²) naive; O(N·S·len) incremental |
| Sum of independent parts | `np.convolve` | O(n·m) |
| Hit/miss/crit mixture | weighted sum of PMFs: `P_miss·G + P_nc·H + P_c·C` | O(n) |
| "Roll twice, keep either" (Savage Attacker) | `CDF²` differenced | O(n) |
| Resistance | map `x → floor(x/2)` **per damage instance** before summing | O(n) |
| Kill / overkill | cap the support at target HP, folding the tail into an absorbing "≥HP" bin | shrinks n |

**Why direct convolution rather than FFT?** Supports stay in the hundreds; a 10-round fight is 901 values. Direct convolution of the worked example's 10-round sum took 0.07 ms. An FFT version agreed to 5×10⁻¹⁸ but returned 255 slightly negative probabilities, which must be clipped and renormalised and which break exact equality tests. FFT only pays off at supports in the thousands, for example a 20-round combat of a level-20 nova build. Even then, an exact alternative is exponentiation by squaring for identical rounds, or capping at HP.

**Precision.** Float64 has ~1e-16 relative error per operation, and probabilities never cancel, so errors stay near machine epsilon after thousands of convolutions. Exact rationals, the route Icepool takes with "Exact fractional probabilities using Python `int`s" ([icepool README](https://github.com/HighDiceRoller/icepool)), are possible but unnecessary for display. They are useful in **golden tests**.

### 2.2 How AnyDice, Troll and successors do it

- **AnyDice** (closed source, by Jasper Flick, per the [dyce "other efforts" list](https://dycelib.org/latest/)) shows "the exact odds of each individual value" ([AnyDice introduction](https://anydice.com/docs/introduction/)). When a die is passed to a function, "the function will be invoked for all possible sequences that can be made by rolling those dice", and recursion is bounded by a configurable "maximum function depth" ([AnyDice functions docs](https://anydice.com/docs/functions/)). In other words it enumerates outcomes exactly; it does not simulate.
- **Troll** (Torben Mogensen) is a "dice roller and probability calculator" that produces each outcome's probability and accumulated probability. Its homepage says some calculations are limited "to avoid stressing the server" ([Troll homepage](https://hjemmesider.diku.dk/~torbenm/Troll/), [quick reference](https://hjemmesider.diku.dk/~torbenm/Troll/quickRef.html)). The paper is Mogensen, "Troll, a Language for specifying Dice-rolls", ACM SAC 2009, [doi:10.1145/1529282.1529708](https://doi.org/10.1145/1529282.1529708). Its abstract describes "(nearly) exact probability calculations" and says naive calculation is often too slow, so optimisations are applied. *We could not retrieve the full PDF to quote the specific optimisations.*
- **Icepool** (Albert Julius Liu, "Icepool: Efficient Computation of Dice Pool Probabilities", AIIDE 2022, [doi:10.1609/aiide.v18i1.21971](https://doi.org/10.1609/aiide.v18i1.21971)). The abstract states that existing general calculators "resort to exhaustive enumeration of all possible sorted sequences of rolls". Icepool instead uses dynamic programming over outcomes, iterating "'vertically' over the outcomes of dice in a dice pool, rather than 'horizontally' through the dice" ([README](https://github.com/HighDiceRoller/icepool)).
- **dyce** "counts how many ways each outcome can occur, then calculates exact probabilities", using integer-count histograms ([dyce docs](https://dycelib.org/latest/)).

**Implication for us.** None of these tools models *turn structure*: attack slots, once-per-turn riders, resource policies. They are dice algebras. Our engine needs a dice algebra (PMF ops) **plus** a state machine over the turn. Keep-highest pools, Icepool's speciality, appear in 5e only as advantage and Elven Accuracy, where the closed form in §1.2 is enough.

### 2.3 Exact versus Monte Carlo

We measured both on the worked example (exact P(round ≥ 50) = 0.0218).

| Method | Mean (±1 SE) | P(≥50) | Time |
|---|---|---|---|
| Exact PMF | 22.897 (exact) | 0.0218 | 0.04 ms |
| MC n = 10³ | 23.28 ± 0.33 | 0.0230 | 1.1 ms |
| MC n = 10⁴ | 22.97 ± 0.10 | 0.0221 | 1.8 ms |
| MC n = 10⁵ | 22.96 ± 0.03 | 0.0218 | 13 ms |
| MC n = 10⁶ | 22.89 ± 0.01 | 0.0222 | 89 ms |

**Error bars.** The standard error of the mean is `σ/√n`; for a probability it is `√(p(1−p)/n)`. Telling two builds 0.25 DPR apart at 2 SE with σ ≈ 10 per round needs `n ≳ 2·(2·10/0.25)² ≈ 12,800` samples per build, per AC and per level. Over a 16-point AC sweep, 20 levels and thousands of candidate builds, that is far beyond a live budget. Exact computation is also **deterministic**, so rankings never flicker between slider moves. Monte Carlo remains the right tool for (a) cross-checking the exact engine in tests, and (b) scenarios that are genuinely intractable (§3.4).

### 2.4 Two-tier evaluation for search

Pure PMFs are cheap, but a solver evaluates millions of build × AC × level points. **Exact moments** (mean and variance) can be propagated through the same state machine without PMFs: carry `(P, E[X], E[X²])` per state. Mixture and convolution rules are closed form: `E` adds, and `Var` adds for independent parts. Use moments in the search loop, and compute full PMFs, percentiles and kill-time curves only for the shortlisted top-K and for the dashboard.

---

## 3. State-dependent sequences (Markov chains / DP)

### 3.1 The pattern

Treat a turn as a sequence of **attack slots**. Carry a map `state → unnormalised PMF`. An attack slot does this:

```
for (state, pmf) in S:
   (pc, ph, pm) = d20(state.advMode, ...)
   N[state']    += conv(pmf, outcomePMF · weight)   # for each outcome (miss/hit/crit) and each rider decision
```

The total mass across states always sums to 1. At turn end, collapse the states that do not persist (for example "SA used") and keep those that do ("slots left", "Vex pending").

### 3.2 Worked state machines

**Hew (GWM, 2024).** A Bonus Action attack right after a Critical Hit, or after reducing a creature to 0 HP, with a melee weapon (paraphrased from [aidedd GWM](https://www.aidedd.org/feat/great-weapon-master)). The state is "crit yet?" for the Attack-action attacks. P(Hew) = 1 − 0.95² = 0.0975 at AC 16. It adds **+0.90** per round and raises the SD from 8.64 to 10.26.

**Topple vs Graze (same fighter, maul vs greatsword).** Topple forces a Con save against DC 8 + mod + PB (15); on a failure the target is Prone (SRD p.90). Melee attacks within 5 ft of a prone target have Advantage (SRD p.186). The state is "prone?". The target stands on its own turn, so the state resets each round.

| Target Con save | P(prone after a hit) | Maul/Topple mean (SD) | Greatsword/Graze mean (SD) |
|---|---|---|---|
| +1 | 0.65 | 21.53 (13.77) | 22.90 (10.26) |
| +3 | 0.55 | 21.23 (13.62) | 22.90 (10.26) |
| +7 | 0.35 | 20.63 (13.27) | 22.90 (10.26) |

AC sweep (Con +3): at AC 12 the two are equal (27.4 vs 27.5); at AC 20 the gap is 14.5 vs 18.3. **White-room caveat:** a solo-fighter model ignores the advantage that *allies* get against the prone target. This is exactly the kind of hidden party value the spec's "honest probability" principle should expose. Make it a toggle ("allies' attacks benefit: N"), not a default.

**Vex.** "If you hit … and deal damage …, you have Advantage on your next attack roll against that creature before the end of your next turn" (SRD p.90). The state is "adv pending?" and **can cross the turn boundary**, so it must persist between rounds. Rogue example: 16.51 → 17.11.

**Sap** imposes Disadvantage on the *target's* next attack (SRD p.90). It does not affect DPR; it belongs to the mitigation/tank metrics (out of MVP scope) and needs a model of enemy attacks.

**Smite / slots.** The state is `(round, slot index, slots left, smite used this turn)`. A backward DP gives the mean-optimal policy, and a forward PMF propagation under that policy gives its distribution (§1.4 table).

### 3.3 Objective-dependent policies

The policy that maximises the **mean** is not the one that maximises **P(kill within K)**. For the latter, the DP state must include cumulative damage, capped at HP (support ≤ HP+1 ≈ 100–700). That is still tractable: about 10³ resource states × 10³ damage bins × a handful of attack slots per round. The engine must therefore decide which objective defines "the policy" (Candidate ADR 5).

### 3.4 When exact computation stops being tractable

| Feature | State growth | Verdict |
|---|---|---|
| Per-turn flags (SA used, crit seen, prone, Vex) | ×2 each | trivial |
| Resources over a fight (slots by level, Ki/Focus, Rage, Action Surge, Superiority dice) | product of counters, e.g. 5 × 3 × 4 × 2 ≈ 10² | fine |
| Kill-dependent triggers (Hew on kill) | damage is already the PMF index, so split at HP | fine |
| Multiple targets, each with its own HP (Cleave, multi-target) | product of per-target PMFs, 10³ⁿ | intractable beyond 2–3 targets → Monte Carlo |
| Enemy actions (it attacks, moves, stands up or not, makes concentration saves) | adversary policy | Monte Carlo or fixed scripts |
| History-dependent rules (e.g. "first time each turn after X") with long horizons | grows with horizon | usually compressible; otherwise Monte Carlo |

For the MVP's single-target sustained/nova DPR, **everything is exactly tractable.**

---

## 4. Reliability metrics

### 4.1 The spec's Build A vs Build B, made concrete

The spec only gives the shape ("85% hit, low-variance dice, Graze" vs "40% hit, swingy crits"), so we built two concrete stand-ins with equal means:

- **Build A:** 3 attacks, 85% hit (5% crit), `1d8+12` on a hit, `2d8+12` on a crit, Graze 5 on a miss → per attack `0.80·16.5 + 0.05·21 + 0.15·5 = 15.0`; **45.0/round**.
- **Build B:** 2 attacks, 40% hit with crits on 18–20 (15% crit, 25% plain hit), `6d10+11` on a hit, `12d10+11` on a crit, 0 on a miss → per attack `0.25·44 + 0.15·77 = 22.55`; **45.1/round**.

| Metric (per round) | A | B |
|---|---|---|
| Mean | 45.0 | 45.1 |
| SD | 8.4 | 42.2 |
| CV (SD/mean) | 0.19 | 0.94 |
| Downside semideviation | 6.4 | 27.2 |
| p10 / p50 / p90 | 34 / 46 / 55 | 0 / 43 / 102 |
| P(zero damage) | 0% | 36% |
| P(≥ ½·mean) | 99.7% | 64.0% |
| CVaR₁₀ (mean of worst 10% of rounds) | 28.7 | 0.0 |
| P(≥ 90) | 0.0% | 14.4% |

Kill-time against a single target (rounds needed):

| Target HP | A: P(dead by round 1/2/3/4) | B: P(dead by round 1/2/3/4) | E[TTK] A vs B |
|---|---|---|---|
| 60 | 0.02 / 0.99 / 1.00 / 1.00 | 0.34 / 0.65 / 0.84 / 0.93 | — |
| 90 | 0 / 0.54 / 1.00 / 1.00 | 0.14 / 0.46 / 0.71 / 0.85 | 2.46 vs 2.97 |
| 135 | 0 / 0 / 0.54 / 0.99 | 0.03 / 0.22 / 0.47 / 0.68 | 3.47 vs 3.95 |
| 180 | 0 / 0 / 0.00 / 0.53 | 0.00 / 0.08 / 0.26 / 0.48 | 4.48 vs 4.93 |

ASCII sketches (10-point buckets):

```
A  30-39 ###########################           B   0-9  ##################################################  36%
   40-49 ################################################## 39%   40-49 #####################
   50-59 ##########################################           70-89 ###########  (each bucket)
                                                           100-179 ###  (long right tail to 262)
```

### 4.2 Which metric best represents "reliability"?

- **SD and CV** describe spread, but they treat upside and downside alike and do not say what the spread costs. They are fine as secondary chips.
- **Lower quantiles (p10) and CVaR₁₀** capture "how bad is a bad round". They are intuitive ("in your worst 1-in-10 rounds you average 29") and good for a **floor** metric.
- **Time-to-kill (TTK) against the baseline target's HP** is the best single headline. It is in game units (rounds), and it **naturally prices overkill**: B's 262-damage spikes are wasted on a 135-HP monster, so equal mean DPR still gives B about 0.5 rounds slower expected TTK. It also flips correctly when variance *helps*: B is far likelier to delete a 60-HP target in round 1 (34% vs 1.5%), which is what a nova player actually wants.

**Recommendation:** headline = **E[TTK] and P(kill within K rounds)** against the baseline HP (K = 3 for sustained, 1–2 for nova). Secondary = mean DPR, then a reliability band of p10 / p50 / p90 with CVaR₁₀ as the "floor". CV is a single scalar "reliability score" for sorting tables.

---

## 5. Encounter baselines

### 5.1 Published sources

- **2014 DMG "Monster Statistics by Challenge Rating" (DMG pp. 274–275).** Transcription as CSV in the CRAN package `dndR` ([raw CSV](https://github.com/njlyon0/dndR/blob/main/dev/raw_data/quick_monster_dmg.csv); [package docs](https://search.r-project.org/CRAN/refmans/dndR/html/monster_table.html)). AC: 13 (CR ≤3), 14 (4), 15 (5–7), 16 (8–9), 17 (10–12), 18 (13–16), 19 (17–30). Attack bonus: +3 to +14. Save DC: 13 to 23. HP ranges run from 71–85 at CR 1 to 356–400 at CR 20. The table has **no save-bonus column**.
- **Paul Hughes (Blog of Holding), 2014 MM.** The Monster Manual's ACs are "usually within a point" of the DMG's, but its HP are "much lower" than the DMG's ([2018 post](https://www.blogofholding.com/?p=7283)). His "MM on a business card": **AC = 13 + CR/3 (±3)**, attack = 4 + CR/2, good save = 3 + CR/2 ([post](https://blogofholding.com/?p=7338)). For the 2024 MM: "AC has increased across the board by exactly one point"; HP = 15 + 15·CR for CR 1–20 ([2025 post](https://blogofholding.com/?p=8469)).
- **Tom Dunn (The Finished Book).** Baseline (2014 official books): **AC ≈ 13 + CR/3, SB ≈ CR/2 (mean of six saves), AB ≈ 3.5 + CR/2, HP ≈ 16 + 16·CR**, plus a per-CR table ([baseline monster stats](https://tomedunn.github.io/the-finished-book/monsters/baseline-monster-stats)). He reports PC-DC failure rates of about 65% for Dex and Int saves and about 58% for Str, while Con falls from about 50% toward 40%. Comparing MM 2024 with MM 2014, he finds "slightly higher AC" but "insufficient evidence to suggest a change in the baseline", and saves growing at **≈ 1.0 + CR/3** ([MM 2024 analysis](https://tomedunn.github.io/the-finished-book/monsters/monster-manual-2024)). He also covers per-ability save patterns ([monster saving throws](https://tomedunn.github.io/the-finished-book/monsters/monster-saving-throws)).
- **Yaarel (EN World), 2024 MM.** "On average, the AC in the 2024 Monster Manual improves by 0.42 of a point for each CR" ([post](https://www.enworld.org/goto/post?id=9573967)). Attack bonus rises about 0.548 per CR ([thread](https://www.enworld.org/posts/9572810/)).
- **Teos Abadía (Alphastream).** A stats-by-CR table averaged over "every single monster in the 2024 MM", smoothed and cross-checked against Hughes; published as an image ([post](https://alphastream.org/index.php/2025/03/26/how-to-create-a-monster-for-revised-dd-5e-2024/)). The 2024 DMG dropped the 2014 creation table, according to the same post and [Blog of Holding](https://blogofholding.com/?p=8469).

### 5.2 Our own check: SRD 5.2.1 stat blocks (2024-era monsters)

We parsed all 330 creature stat blocks with a CR from the [SRD 5.2.1 PDF](https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf) (CC-BY-4.0; 2 of 332 blocks skipped). For each we took AC, HP, CR, the six SAVE columns, and whether it has the Magic Resistance trait.

| CR | n | mean AC | SD AC | AC range | Str | Dex | Con | Int | Wis | Cha | mean of 6 | best save | Magic Res. | mean HP |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 0–1/2 | 108 | 12.0 | 1.9 | 5–18 | −0.3 | +1.2 | +0.6 | −3.0 | +0.2 | −2.3 | −0.6 | +2.2 | 2% | 12 |
| 1–2 | 69 | 13.1 | 2.3 | 6–19 | +2.2 | +1.8 | +1.6 | −2.0 | +0.7 | −1.0 | +0.5 | +3.4 | 6% | 41 |
| 3–4 | 41 | 14.5 | 2.3 | 7–19 | +3.2 | +1.4 | +2.4 | −1.0 | +1.1 | +0.2 | +1.2 | +4.0 | 2% | 70 |
| 5–6 | 35 | 15.2 | 2.2 | 9–20 | +3.7 | +1.7 | +3.7 | −1.2 | +1.6 | −0.2 | +1.6 | +5.0 | 14% | 106 |
| 7–8 | 16 | 16.0 | 1.8 | 12–18 | +4.8 | +2.8 | +4.7 | +0.2 | +2.9 | +0.9 | +2.7 | +6.0 | 12% | 132 |
| 9–10 | 14 | 16.9 | 1.4 | 14–18 | +6.1 | +2.7 | +5.9 | +2.1 | +4.6 | +3.6 | +4.2 | +7.3 | 36% | 170 |
| 11–12 | 9 | 17.0 | 0.8 | 15–18 | +5.7 | +3.7 | +4.9 | +1.3 | +4.2 | +3.7 | +3.9 | +7.6 | 56% | 195 |
| 13–14 | 9 | 17.6 | 1.0 | 16–19 | +6.0 | +4.9 | +7.1 | +3.0 | +6.4 | +5.7 | +5.5 | +8.2 | 33% | 201 |
| 15–16 | 9 | 18.3 | 1.2 | 16–20 | +7.7 | +3.1 | +7.2 | +1.9 | +6.6 | +4.1 | +5.1 | +8.7 | 44% | 224 |
| 17–20 | 8 | 19.4 | 1.1 | 17–21 | +7.6 | +5.2 | +8.8 | +3.6 | +8.5 | +5.1 | +6.5 | +9.6 | 25% | 293 |
| 21–30 | 12 | 21.5 | 1.6 | 18–25 | +8.9 | +7.8 | +8.9 | +5.2 | +9.4 | +7.0 | +7.9 | +10.7 | 17% | 448 |

Least-squares fits over CR ≥ 1 (n = 222): **AC = 12.90 + 0.374·CR**; HP = 9.9 + 17.0·CR; mean-of-six save = −0.01 + 0.355·CR. Per-ability save fits: Str +1.93 + 0.331·CR; Dex +0.88 + 0.250·CR; Con +1.33 + 0.377·CR; Int −2.41 + 0.338·CR; Wis −0.21 + 0.436·CR; Cha −1.55 + 0.398·CR.

Caveats: the SRD is a subset of the 2025 MM (high-CR bins have n ≤ 12). Magic Resistance was detected by trait name. Legendary Resistance is not modelled. These figures agree with Dunn and Yaarel within about 0.5 AC.

### 5.3 Recommended default target curve (CR = character level)

PC attack bonus assumes 27-point buy with 15 + 2 = 17 in the primary stat, raised to +4 at L4 and +5 at L8. The PC DC is 8 + PB + mod.

| Lvl | DMG 2014 AC | Dunn AC | SRD fit | **Default AC** | PC atk | P(hit) | SRD avg save | PC DC | P(fail, avg) |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 13 | 14 | 13.3 | **14** | +5 | 60% | +0.3 | 13 | 58% |
| 3 | 13 | 15 | 14.0 | **15** | +5 | 55% | +1.1 | 13 | 55% |
| 5 | 15 | 15 | 14.8 | **15** | +7 | 65% | +1.8 | 15 | 61% |
| 8 | 16 | 16 | 15.9 | **16** | +8 | 65% | +2.8 | 16 | 61% |
| 11 | 17 | 17 | 17.0 | **17** | +9 | 65% | +3.9 | 17 | 61% |
| 13 | 18 | 18 | 17.8 | **18** | +10 | 65% | +4.6 | 18 | 62% |
| 16 | 18 | 19 | 18.9 | **19** | +10 | 60% | +5.7 | 18 | 57% |
| 17 | 19 | 19 | 19.3 | **19** | +11 | 65% | +6.0 | 19 | 60% |
| 20 | 19 | 20 | 20.4 | **20** | +11 | 60% | +7.1 | 19 | 55% |

Default AC = Dunn's baseline table (≈ 13 + CR/3, rounded as he tabulates). For all 20 levels it lies within ±0.6 of our SRD 5.2.1 fit. RPGBot's DMG-based curve gives a constant ~65% hit rate ([RPGBot fundamental math](https://rpgbot.net/dnd5/characters/fundamental_math.html)); ours sits a little lower (55–65%) at levels where the DMG is generous.

**Save profile:** use **per-ability** curves (the fits in §5.2) plus the Magic Resistance share of each CR band. Applied at PC DCs, the SRD data gives failure rates such as (L11, DC 17): Str 52%, Dex 62%, Con 56%, Int 73%, Wis 59%, Cha 62%. With MR applied, these fall to 39–62%. A single "average save" hides a spread of about 20 percentage points between the best and worst save to target.

**HP for TTK:** HP ≈ 10 + 17·CR (SRD fit), close to Dunn's 16 + 16·CR and Hughes' 2024 15 + 15·CR. The DMG 2014 ranges are too high, as Hughes found.

---

## 6. Prior art and common pitfalls

| Tool / source | Method (as documented) | Notes |
|---|---|---|
| LudicSavant & AureusFulgens, "Comprehensive DPR Calculator" (GiantITP, v2.x, ~2019) ([thread](https://forums.giantitp.com/showthread.php?p=23761260), [sheet](https://docs.google.com/spreadsheets/d/1Qv3skx5dwgq1piGZq6gzfZb6N6XRj99CEto2MAXrWEk/edit)) | Spreadsheet of expected values; Normal/Adv/Disadv columns; baseline built on the DMG adventuring day (per the developers' forum reply) | Mean only. We could not load the thread directly (HTTP 403); this is from search results. |
| RPGBot (Tyler Kamstra) ([DPR calculator](https://rpgbot.net/dnd5/tools/dpr-calculator), [fundamental math](https://rpgbot.net/dnd5/characters/fundamental_math.html)) | Mean DPR. Assumes a 65% hit target, AC from the DMG table with CR = level, 3-round encounters for a party of 4, target DPR = max HP / 12 | A good baseline philosophy. An RPGBot update post notes it handles once-per-turn boosts differently from LudicSavant's sheet. |
| DPRcalc ([Patreon post](https://www.patreon.com/DPRcalc/posts/introducing-math-173769846)) | Reports "average damage and expected damage distribution, with stats", with dice notation for simulated attacks | Page returned 403 to us; claims taken from the search-result excerpt. |
| Generic web calculators (e.g. [hakaru](https://hakaru.io/guides/dnd-damage-calculator-guide), [everycalculators](https://everycalculators.com/dpr-calculator-5e.html)) | `DPR = P(hit)·avg + P(crit)·extra` | everycalculators' worked example contains an arithmetic error (0.65 × 8.725 is quoted as 5.84; it is 5.67). |
| Treantmonk's baselines (critiqued in [EN World threads](https://www.enworld.org/goto/post?id=9535581)) | Fixed hit rate, many encounters | Critics argue the hit-rate and encounter-count assumptions penalise spike builds. |

**Common white-room pitfalls the engine should guard against:**

1. A fixed hit rate (e.g. 65%) instead of an AC sweep; this hides accuracy/damage trade-offs. We show the full AC 10–25 curve.
2. Permanent advantage. Make it an explicit toggle and show both values.
3. Doubling flat bonuses on crits, or forgetting to double rider dice (SA, Smite). See SRD p.16.
4. Applying a once-per-turn rider to every attack, or the wrong `1 − Πm` with crits ignored (§1.4).
5. Halving the *mean* for resistance or save-for-half. Floors apply per damage instance, so transform the PMF instead.
6. Ignoring overkill. Mean DPR overstates spiky builds against realistic HP pools (§4.1).
7. Counting ally benefit nowhere (Topple prone, Vex) or everywhere. It needs to be an explicit knob.
8. Treating "nova" as round 1 with every resource but no resource accounting across a day.
9. Using DMG 2014 HP ranges as monster HP; real monsters have notably less (Hughes; SRD fit).

---

## 7. Algorithms, data structures and the worked example

### 7.1 Recommendations

- **PMF type:** `{offset: int, p: Float64Array}` with helpers `die(s, {gwf})`, `conv`, `shift`, `mix`, `maxOf2`, `mapFloor(f)`, `capAt(hp)`, `mean`, `var`, `quantile`, `tailGE(x)`.
- **Truncation:** none needed for single rounds. For long fights, either cap at target HP (exact for kill metrics) or drop tail mass below 1e-12 while recording the dropped mass so tests can assert `|1 − Σp| < 1e-9`.
- **Caching:** memoise dice PMFs by canonical key (`"2d6:gwf"`); memoise attack-outcome PMFs by `(dice key, flat, critDiceKey)`. These are **independent of AC**, so an AC sweep only re-mixes weights. Memoise `d20(B−AC, critOn, mode)` (at most 3 × 41 × 3 entries).
- **Turn engine:** a state machine over attack slots carrying `Map<StateKey, PMF>` (or `(P, E, E²)` in moment mode), with explicit collapse rules at turn and round boundaries. Resource policies come from a backward DP over the same states.
- **Testing:** golden tests against hand-derived closed forms (the numbers in this note), plus a seeded Monte Carlo oracle within 4 SE.
- **Worker:** run the engine in a Web Worker; the PMF arrays are small enough to post back as transferable `Float64Array`s.

### 7.2 Worked example: level-5 2024 Fighter, greatsword, GWM, against AC 16

**Assumptions.** Soldier-style background (+2 Str); Str 15 → 17 → 18 with GWM's +1; PB +3, so attack +7. Great Weapon Fighting style (2024: 1s and 2s count as 3, so E[d6] = 4). Greatsword 2d6 Slashing, Heavy, Graze mastery (SRD p.91). GWM Heavy Weapon Mastery adds +PB (3) on hits **made as part of the Attack action**. Hew is a bonus-action attack after a crit and does **not** get +PB, since it is not part of the Attack action (Open Q2). Extra Attack gives 2 attacks. No Action Surge in the sustained round. Savage Attacker is excluded from the base case.

Per attack: `P_crit = 0.05, P_hit-noncrit = 0.55, P_miss = 0.40`.
`E[hit] = 8 + 4 + 3 = 15`, `E[crit] = 16 + 7 = 23`, `Graze = 4`.
`E[attack] = 0.55·15 + 0.05·23 + 0.40·4 = 8.25 + 1.15 + 1.60 = 11.00` → two attacks = 22.00.
Hew: `P(≥1 crit) = 1 − 0.95² = 0.0975`; Hew attack `E = 0.55·12 + 0.05·20 + 0.40·4 = 9.20` → `+0.897`.
**Mean = 22.897 per round** (the exact PMF agrees).

**Distribution (exact):** SD **10.26**, CV 0.45; min 8, p10 8, p25 17, **median 20**, p75 30, p90 34, p95 42, max 90.
P(round ≥ 10 / 20 / 30 / 40 / 50) = 0.840 / 0.571 / 0.261 / 0.064 / 0.022.

```
dmg     prob   (5-point buckets)
  5-9   16.0%  ##############################      <- both attacks graze: exactly 8 (0.40²)
 15-19  26.9%  ##################################################   <- one hit + one graze (17–20)
 20-24  17.1%  ################################
 25-29  13.9%  ##########################          <- two hits (26–34)
 30-34  16.9%  ###############################
 35-39   2.8%  #####
 40-49   4.2%  ########                            <- crits / Hew
 50-64   2.1%  ####
```

**What each component is worth at AC 16 (mean / SD):**

| Variant | Mean | SD |
|---|---|---|
| Base (GWF + GWM + Graze + Hew) | 22.90 | 10.26 |
| No Hew | 22.00 | 8.64 |
| No Graze | 19.54 | 12.49 |
| No GWM (ASI instead, same Str mod) | 18.40 | 6.74 |
| No GWF | 21.53 | 9.57 |
| Hew counted with +PB (alternative reading) | 23.07 | 10.68 |
| + Savage Attacker (greedy first hit) | 23.69 | 10.51 |
| Permanent advantage | 30.17 | 10.68 |
| Nova round (Action Surge: 4 attacks + Hew) | 45.71 | 14.29 |

**AC sensitivity (mean per round).** At AC 10 / 13 / 16 / 19 / 22 / 25: base **29.7 / 26.3 / 22.9 / 19.5 / 16.1 / 12.7**; without Graze 28.9 / 24.2 / 19.5 / 14.9 / 10.2 / 5.5. The 2024 GWM has no accuracy penalty, so it is never switched off. Graze's value grows from 0.8 at AC 10 to 7.1 DPR at AC 25. This is the "Graze overtakes" crossover the spec's AC-sensitivity chart is meant to show.

**3-round fight** (Action Surge in round 1): mean 91.5, SD 20.4, p10 65, p90 118. Against a CR 5 baseline target of ~90 HP alone: P(dead by end of round 2 / 3 / 4) = 0.12 / 0.51 / 0.87, E[TTK] = 3.5 rounds.

---

## Open questions

1. **Crit range against an impossible AC.** The 2024 glossary attaches "hits regardless of AC" to the natural 20 (SRD p.179). Does a Champion's natural 19 hit when 19 + B < AC? We assume yes (crit implies hit). It matters only when `AC − B > 19`. Needs a ruling ADR or a toggle.
2. **Hew and Heavy Weapon Mastery.** Hew is a Bonus Action and HWM requires "part of the Attack action", so our reading is no +PB on Hew. The difference is 0.18 DPR at L5. The rules-text source is a secondary transcription (aidedd), because GWM is not in SRD 5.2.1.
3. **Default target level offset.** Is CR = level right for "single-target"? A boss for a party of four is typically higher CR. Offer presets (CR = L, L+2, L+4)?
4. **2024 AC: +0 or +1?** Hughes says +1 across the board, Dunn says no change, Yaarel finds a 0.42/CR slope, and our SRD subset fits 12.9 + 0.37·CR. A full 2025 MM dataset (not CC-licensed) would settle it.
5. **Ally modelling.** How should Topple, Vex-style or party-wide buffs be credited without the "everything assumes a perfect party" white-room error?
6. **Objective for resource policies.** Mean-optimal, or P(kill)-optimal? They differ (§3.3).
7. **Adventuring-day model for "sustained".** How many short rests, and how many fights between them? This determines how much of the slot, Action Surge and Focus budget is available per fight. A separate research topic.

## Candidate ADRs

1. **Exact discrete PMFs are the engine's source of truth; Monte Carlo is only a test oracle and a fallback for intractable scenarios.**
   *Decision:* every displayed probability, percentile and curve comes from exact float64 PMF propagation through a turn state machine.
   *Rationale:* supports are small, so exact computation costs ≈0.04 ms per round versus ≈13 ms for ±0.03 MC. It is deterministic, so solver rankings and slider updates never jitter. Tails such as P(kill in round 1) are exact.
   *Rejected:* Monte Carlo everywhere (noise, cost of ~10⁴ samples per comparison point, flickering ranks); FFT convolution (no speed-up at these sizes, negative-probability artefacts); exact rationals at runtime (slow; kept for tests only).

2. **The solver searches on exact moments; full distributions are computed only for the shortlist and the dashboard.**
   *Decision:* the search loop propagates `(P, E, E²)` per state; PMFs, quantiles and TTK are computed for the top-K builds.
   *Rationale:* the build × level × AC grid has millions of points. Moments are exact and microseconds-cheap.
   *Rejected:* full PMFs for every candidate (100× cost); mean-only (cannot express the spec's reliability ranking or variance constraints).

3. **Default encounter baseline: target CR = character level; AC from Dunn's baseline table (≈ 13 + CR/3); per-ability save curves and Magic Resistance share fitted to SRD 5.2.1; HP ≈ 10 + 17·CR.**
   *Rationale:* three independent analyses (Hughes, Dunn, Yaarel) and our SRD 5.2.1 fit agree within about 0.5 AC. The data are CC-BY, so it can be shipped and reproduced. Per-ability saves expose the roughly 20-point spread in failure rate between abilities.
   *Rejected:* the DMG 2014 table (AC too flat at CR 1–3 and 17–30, HP far too high, no save bonuses); a fixed 65% hit rate (hides the AC trade-off); a single average save bonus.

4. **"Reliability" is headlined as time-to-kill and P(kill within K rounds) against the baseline HP, with a p10/CVaR₁₀ floor shown alongside; SD and CV are secondary.**
   *Rationale:* it is in game units, it prices overkill, and it correctly rewards variance for nova and penalises it for sustained play (Build A vs B: E[TTK] 3.47 vs 3.95 at 135 HP; P(one-round kill of 60 HP) 1.5% vs 34%).
   *Rejected:* SD alone (symmetric and unitless as a cost); CV as the headline (fine for sorting but uninterpretable); a bespoke weighted "reliability score" (opaque).

5. **Resource and rider decisions use the mean-optimal policy found by DP; distributions are reported under that policy.**
   *Rationale:* fixed heuristics are measurably wrong in some cases (crit-only smiting loses 5–19 damage per fight). Using one policy per build keeps every chart consistent.
   *Rejected:* hard-coded heuristics; a different optimal policy per metric (charts would describe different play patterns); letting the user script policies (deferred).

## Glossary candidates

| Term | Definition | Avoid |
|---|---|---|
| **Damage PMF** | The exact probability mass function of integer damage for an attack, turn, round or combat. | "damage curve", "histogram" (implies sampling) |
| **Attack outcome** | One of Miss, Hit or Crit for a single attack roll, each with its own damage PMF. | "result", "roll result" |
| **Graze floor** | Damage dealt on a miss by the Graze mastery (the ability modifier); it lower-bounds the round PMF. | "miss damage", "chip damage" |
| **Rider** | Extra damage applied at most once per turn to one hit (Sneak Attack, Divine Smite, Savage Attacker). | "proc", "bonus damage" |
| **Turn state** | The per-attack-slot condition (advantage pending, prone, rider used, crit seen, resources) that the engine propagates. | "context", "flags" |
| **Policy** | The rule deciding resource and rider use given the turn state; by default the DP mean-optimal one. | "strategy", "AI" |
| **Sustained DPR** | Mean damage per round over a standard fight without nova resources beyond the per-fight budget. | "average DPR" |
| **Nova DPR** | Mean damage in rounds 1–2 with burst resources spent. | "burst", "alpha strike" |
| **Encounter baseline** | The default target's AC, per-ability saves, Magic Resistance share and HP as a function of CR (= level by default). | "target dummy stats", "standard monster" |
| **Target dummy** | The single configurable target the dashboard evaluates against (baseline plus overrides). | "enemy", "boss" |
| **AC sensitivity curve** | Mean (and band) damage per round plotted over target AC 10–25. | "AC graph" |
| **Time-to-kill (TTK)** | The distribution of the number of rounds until cumulative damage reaches the target's HP. | "rounds to kill" (as a single number) |
| **Kill probability P(kill ≤ K)** | The probability that cumulative damage reaches the target HP within K rounds. | "lethality" |
| **Reliability band** | p10–p50–p90 of per-round damage, shown with CVaR₁₀. | "variance bar", "error bar" |
| **CVaR₁₀ (floor)** | Mean damage of the worst 10% of rounds. | "minimum damage" (that is p0) |
| **White-room assumption** | A modelling simplification (fixed hit rate, permanent advantage, no overkill, no allies) not stated as a toggle. | "theorycraft" |
