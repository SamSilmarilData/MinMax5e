# MinMax5e

A build solver for fifth-edition tabletop RPG characters: it searches the space of legal character builds under user constraints and ranks them by realistic, probability-grounded damage output.

## Rulesets and content

**Ruleset**:
One of two mutually exclusive rule environments, `2024` (primary) or `2014` (legacy). Defined by its core rules and action economy, it scopes every piece of content and every evaluation. A Ruleset covers that edition's full published rules, not only the SRD subset.
_Avoid_: edition mode, version, 5.5e, One D&D

**Rules Pack**:
A bundle of content and its encodings for exactly one Ruleset.
_Avoid_: dataset, corpus, module, book

**Content ID**:
The stable identity of a content item: its Ruleset, kind, name slug, and source (e.g. `2024:feat/great-weapon-master|xphb`).
_Avoid_: content key, slug, uuid

**Corpus**:
The pinned local copy of 5e.tools data that Rules Packs are imported from.
_Avoid_: dataset, dump, mirror

**Upstream record**:
One entry in the Corpus, identified by its `Name|SOURCE` key and a content hash.
_Avoid_: raw entry, source record

**Stale encoding**:
An Encoding whose Upstream record changed after a Corpus version bump and has not yet been re-reviewed.
_Avoid_: outdated, broken

**Encoding**:
The project-authored, executable description of what one content item does mechanically, containing no copied rulebook text.
_Avoid_: script, automation, parsed feature, rule

**Provenance**:
Where a content item came from: SRD, the Corpus (with its source book), or project-authored.
_Avoid_: source (clashes with book source codes), origin (clashes with Origin feats)

**Citation**:
Where an encoded feature's mechanics come from (a section or page reference, never the rules text), with its Source tier (ADR-0007).
_Avoid_: reference, attribution

**Source tier**:
A Citation's place in the order of trust (ADR-0007): errata, SRD, Corpus, or synthetic for fixtures.
_Avoid_: trust level, priority

**SRD content / Fan content**:
Material usable under CC-BY-4.0 (SRD 5.2.1 / 5.1), as opposed to other publisher material used only under the Fan Content Policy.
_Avoid_: open content, official content

**Legacy-compatible**:
A flag marking a 2014-era item that was never reprinted for 2024 as allowed in the 2024 Ruleset, interpreted under 2024 rules.
_Avoid_: backward compatible, legacy mode

**Ruling**:
A named, versioned, cited choice between interpretations of ambiguous rules text. The user can toggle the disputed ones, and every result records the Rulings it used.
_Avoid_: house rule (that means changing the rules), assumption, errata, table ruling

## Game mechanics

**Weapon Mastery**:
The class or feat feature that lets a character use the mastery property of specific weapon kinds.
_Avoid_: mastery slots

**Mastery property**:
A weapon's property (Cleave, Graze, Nick, Push, Sap, Slow, Topple, Vex) that only Weapon Mastery of that weapon kind unlocks.
_Avoid_: mastery, weapon trait

**Mastery Chain**:
A sequence of attacks within one turn using different weapons, whose mastery properties feed each other (e.g. Vex, then Nick).
_Avoid_: swap combo, mastery loop

**Attack action**:
The action whose attacks (including Extra Attack) each allow one equip or unequip.
_Avoid_: main attack, Main Attack action

**Attack source**:
The rule granting an attack (Attack action, Light, Nick, Bonus Action feature, Reaction, Cleave), which determines which riders and limits apply.

**Light extra attack**:
The extra attack the Light property grants, made with a different Light weapon after an Attack-action attack with a Light weapon.
_Avoid_: off-hand attack, TWF attack, bonus attack

**Nick attack**:
A Light extra attack made as part of the Attack action through the Nick property, without using the Bonus Action.
_Avoid_: free off-hand

**Equip token**:
The single equip-or-unequip allowed with each attack made as part of the Attack action.
_Avoid_: swap, free draw

**Free object interaction**:
The one interaction with an object per turn that costs neither an action nor movement.
_Avoid_: free action

**Rider**:
Extra damage or an effect attached to a hit, often limited to once per turn (Sneak Attack, Divine Smite, Hunter's Mark, Rage damage).
_Avoid_: proc, on-hit bonus

**Graze damage**:
Damage equal to the attack's ability modifier dealt on a miss with a Graze weapon.
_Avoid_: miss floor, chip damage

**Hew**:
The 2024 Great Weapon Master Bonus Action attack after a melee crit or after reducing a creature to 0 HP.
_Avoid_: GWM cleave, GWM bonus attack

**One-slot rule**:
The 2024 rule that at most one spell slot may be spent casting spells on any single turn.
_Avoid_: one spell per turn, bonus action spell rule (that is the 2014 rule)

**Slot-free cast**:
Casting a leveled spell through a feature without expending a slot, so the one-slot rule does not count it.
_Avoid_: free cast, free-action spell

**Origin feat**:
A level 1 feat granted by a background.
_Avoid_: background feat, 1st-level feat

**Epic Boon**:
The feat category granted at level 19.
_Avoid_: capstone feat

**Turn / Round**:
A turn belongs to one creature, and a round is every creature's turn. Once-per-turn limits reset on every creature's turn.
_Avoid_: using "round" for once-per-turn effects

## Builds and play

**Build**:
A complete, legal set of character choices: species, background and Origin feat, ability scores, class levels in order, subclasses, feats, spells, and Loadout.
_Avoid_: character, sheet, template

**Build state**:
A Build's choices as of one level, regardless of the order in which they were made.
_Avoid_: sub-build, partial build

**Combat Kit**:
The combat-relevant projection of a Build state (attack bonuses, damage, riders, resources, actions, masteries) that a Rotation is solved over.
_Avoid_: stat block, compiled character

**Loadout**:
The weapons, armor, and shield a Build carries, and the mastery assignments for those weapons.
_Avoid_: gear set, kit

**Choice**:
A decision with a defined set of options, scoped to the Build, each long rest, or each turn.
_Avoid_: pick, selection, option

**Policy**:
The rule that makes per-turn decisions (whether to smite, which weapon to swing) given the current situation in combat.
_Avoid_: strategy, AI, tactic

**Rotation**:
The Policy applied across a combat's rounds for one Combat Kit and Encounter Profile.
_Avoid_: combo, priority list

**Turn Plan**:
One turn of a Rotation: legal steps whose later choices may depend on earlier hit, miss, or crit outcomes.
_Avoid_: combo, sequence

**Combat Script**:
The human-readable presentation of a Rotation: Pre-Combat Setup, Round 1 Opener, Rounds 2+ Sustained Loop, and Reaction priorities.
_Avoid_: workflow, turn guide

## Encounters and metrics

**Target Dummy**:
The single modelled enemy the dashboard evaluates against: AC, per-ability saves, resistances, Magic Resistance, and HP.
_Avoid_: enemy, monster, boss

**Encounter Baseline**:
The default Target Dummy stats as a function of CR, with CR set to character level by default.
_Avoid_: standard monster, white room

**Encounter Profile**:
The Target Dummy plus fight parameters: rounds, advantage state, and target count.
_Avoid_: scenario, encounter (alone)

**Adventuring Day Model**:
The assumed number of encounters, rounds per encounter, and rest schedule used to budget resources for Sustained DPR.
_Avoid_: rest frequency, day

**Assumption Set**:
The named, shareable bundle of Encounter Profile, Adventuring Day Model, and active Rulings that a result was computed under.
_Avoid_: scenario, profile

**White-room assumption**:
A modelling simplification (fixed hit rate, permanent advantage, no overkill) that is not stated as a visible, changeable assumption.
_Avoid_: theorycraft

**DPR**:
Expected damage per round against a single target under a stated Assumption Set: the expected total over the mode's rounds, divided by the number of rounds.
_Avoid_: DPS, damage output

**Sustained DPR**:
DPR averaged over a combat, spending resources only within the Adventuring Day Model's budget.
_Avoid_: average DPR, at-will DPR

**Nova DPR**:
DPR over rounds 1–2 with burst resources spent freely.
_Avoid_: burst, alpha strike, spike

**SD**:
The standard deviation reported beside a DPR: the SD of the total over the same rounds, divided by the number of rounds. A single round's SD is the SD over a one-round horizon.
_Avoid_: spread (that is the Reliability Band), variance (its square)

**Attack outcome**:
One of Miss, Hit, or Crit for a single attack roll.
_Avoid_: roll result

**Damage Distribution**:
The exact probability of every possible damage total for an attack, turn, round, or combat.
_Avoid_: damage curve, histogram (implies sampling)

**Time-to-kill**:
The distribution of the number of rounds until cumulative damage reaches the target's HP.
_Avoid_: rounds to kill (as a single number)

**Kill probability**:
The probability that cumulative damage reaches the target's HP within K rounds.
_Avoid_: lethality

**Floor**:
The mean damage of the worst 10% of rounds (CVaR₁₀), or Graze-style damage that is guaranteed.
_Avoid_: minimum damage (that is the true minimum)

**Reliability Band**:
The displayed spread of a Build's damage (p10, p50, p90, and Floor) beside its mean.
_Avoid_: variance bar, error bar, confidence interval

**AC Sensitivity Curve**:
Expected damage per round plotted against target AC 10–25 for each shown Build.
_Avoid_: AC graph, hit curve

**Crossover AC**:
The target AC at which one Build's expected damage overtakes another's.
_Avoid_: breakpoint (reserved for discrete feature thresholds)

**Damage Source Attribution**:
The breakdown of expected damage into source categories (weapon, mastery, feat, class or spell rider), computed by a stated method.
_Avoid_: damage breakdown, pie

## Solving

**Solve**:
A search of the Build space for the highest-ranked Builds under an objective and an Assumption Set. A Solve is cancellable, and a newer one replaces it.
_Avoid_: optimise, generate, query

**Evaluate**:
Scoring a Build the user supplies, without searching.
_Avoid_: calculator mode

**Candidate**:
A Build produced during a Solve that has not yet been ranked.
_Avoid_: option, combo

**Candidate pool**:
The set of Candidates kept from a Solve (in memory, or in the disk cache for one Partition) that is guaranteed to contain the top Builds at every encounter grid point.
_Avoid_: cache, shortlist

**Encounter grid**:
The finite set of encounter points a Candidate pool is exact over: target AC 10–25 × advantage state × target resistance × round count.
_Avoid_: scenario space, sweep

**Lens**:
A way of ranking the Candidate pool (Expected, Reliability, or Robust) that re-orders Builds without changing any Rotation.
_Avoid_: objective (that is what the Rotation maximises), mode, filter

**Partition**:
The slice of the Build space defined by level, class combination, mode, and Adventuring Day Model for which a Candidate pool is computed.
_Avoid_: bucket, shard

**Share Link**:
A URL that reproduces a result (Assumption Set, constraints, and optionally Builds by Content ID) on the recipient's own data. It carries no rules text.
_Avoid_: permalink, export

**Invite**:
A personal, revocable link that gives one friend access to the hosted app.
_Avoid_: account, login, password

**Build Signature**:
The coarse identity (class split, subclasses, weapon style, key feats) used to decide whether two Builds are meaningfully different.
_Avoid_: archetype, family

**Sidegrade**:
A Build with the same Build Signature as a shown Build and a score within tolerance of it, collapsed under that Build's card.
_Avoid_: variant, alt

**Level score**:
A Build's objective value at one level, normalised by the best value achievable at that level under the same constraints.
_Avoid_: relative power, rating

**Smooth Leveling Curve**:
The objective over Level scores across a level range that penalises power troughs.
_Avoid_: progression score, curve mode

**Peak Target-Level Power**:
The objective that ranks Builds only by their score at the chosen level.
_Avoid_: level 20 snapshot

## Evaluation engine

**Kit Shape**:
The structure of a Combat Kit that determines its rotation state graph (attack sources, Rider slots, resource declarations, swaps, the Rulings it reads), without its numbers.
_Avoid_: template, archetype

**Kit Params**:
The numeric part of a Combat Kit (to-hit, damage-term moments, DCs), evaluated over a fixed Kit Shape.
_Avoid_: stats

**Tape**:
The recorded rotation state graph of one Kit Shape. It is traced once, reduced, and then read in different algebras (moments, occupancy, distributions).
_Avoid_: trace (the verb that makes it), bytecode

**Lane**:
One independent evaluation inside a Tape pass: one kit at one target AC.

**Spend cap**:
The most units of a resource that can be spent in one turn. Together with the turns remaining, it bounds which resource states can be told apart.

**Factor class**:
The set of values of one Build factor (species, origin, ability array, …) that project to the same kit contribution. Its members are DPR-neutral alternatives.
_Avoid_: equivalence group

**Occupancy**:
The probability of reaching each state, and the expected number of uses of each damage term and resource, under a fixed Policy.
_Avoid_: visit counts, heat map

**Sensitivity**:
The exact change in expected damage per unit change of one Kit Param, read from Occupancy.
_Avoid_: stat weight, scale factor (those are SimulationCraft's simulated estimates)

**Script rule**:
The simplest play rule within the Combat Script tolerance of the optimal Policy, shown with its DPR cost.
_Avoid_: priority list, APL

**Optimality gap**:
The certified bound on how far a Solve's best Build could be from the true optimum: 0 means proven optimal.
_Avoid_: error, confidence
