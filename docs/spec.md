# MinMax5e: Project Specification

> **Status: original concept spec (Draft v1.0, November 2026), kept as written for history.** It predates the research in [`research/`](research/README.md) and the decisions in [`adr/`](adr/). Where they disagree, **the ADRs win**. The main differences:
>
> - **Data source:** 5e.tools data is downloaded to each user's machine at a pinned version and never stored in this repo ([ADR-0027](adr/0027-5etools-local-corpus.md), [ADR-0028](adr/0028-no-upstream-bytes-in-public-repo.md), [ADR-0041](adr/0041-in-app-corpus-fetch.md)). Rules prose is not parsed into triggers; mechanics are hand-encoded ([ADR-0002](adr/0002-hand-authored-mechanics.md)).
> - **Scope and stack:** a personal native Rust/egui app, not a hosted product ([ADR-0026](adr/0026-personal-tool-scope.md), [ADR-0032](adr/0032-native-macos-app.md), [ADR-0033](adr/0033-native-rust-engine.md), [ADR-0039](adr/0039-egui-gui.md)). The working title "D&D 5e Build Solver & Optimizer" became **MinMax5e**.
> - **Rules corrections** ([research 02 §7](research/02-rules-mechanics-2024.md)):
>   - "Vex Dagger" is wrong: the Dagger's mastery is Nick.
>   - Spirit Shroud and Booming Blade are 2014-era content, allowed only through the legacy-compatible whitelist ([ADR-0009](adr/0009-legacy-compatible-whitelist.md)).
>   - Hex is a Warlock spell.
>   - The 2024 Monk has no Weapon Mastery.
>   - "Advantage loops" (Vex), "guaranteed floor" (Graze), and "free-action smites" overstate the rules.
> - **Variance:** the solver optimises expected value. Reliability is headlined as time-to-kill and kill chance ([ADR-0017](adr/0017-expected-value-objective.md), [ADR-0025](adr/0025-reliability-as-time-to-kill.md)).
>
> *Unofficial fan project, not affiliated with Wizards of the Coast. "D&D" below refers to the game the tool is compatible with.*

## **Metadata Summary**

| Attribute | Specification Details |
| :---- | :---- |
| Document Status | Draft v1.0 — Conceptual & Functional Specification |
| Primary Ruleset Priority | D\&D 2024 Revised Ruleset (P0 — Core Focus) |
| Secondary Ruleset Support | D\&D 2014 Legacy Ruleset (P1 — Isolated Sandbox) |
| Mechanical Data Source | Complete 5e.tools Corpus (Classes, Subclasses, Species, Feats, Spells, Equipment) |

# **Section 1: Executive Summary & Product Vision**

## **Core Concept**

The **D\&D 5e Build Solver & Optimizer** is an automated mathematical engine and tactical decision-support system for Dungeons & Dragons. Unlike traditional passive character sheets (which function merely as digital ledgers for manual user choices) or static community optimization guides (which rely on rigid, hand-calculated assumptions), this system codifies every game rule, spell, feat, and class feature into executable mechanical logic. By evaluating millions of combinations against rigorous mathematical models, the system solves for the optimal build strategy given a set of user-defined constraints and tactical objectives.

## **Strategic Differentiators**

* **Solving vs. Templating:** Rather than matching users against pre-written build templates, the engine takes concrete constraints—such as target level, playstyle archetype, locked species/class choices, and multiclass toggles—and algorithmically surfaces the mathematically optimal combination of level splits, feat acquisitions, spell selections, and combat action rotations. This enables the discovery of counter-intuitive or novel rule interactions overlooked by static manual theorycrafting.  
* **2024-First Architecture:** Designed from the ground up around the 2024 Revised Ruleset as its primary target (P0). The system natively models complex new mechanical interactions—including dynamic Weapon Mastery swapping sequences, origin-based feat progression, and overhauled action economy constraints—while isolating the legacy 2014 ruleset within a dedicated, secondary sandbox (P1).  
* **Honest Probability Over White-Room Hype:** Most community guides evaluate character options using unrealistic "white-room" assumptions (such as permanent advantage, zero target AC variance, or guaranteed maximum damage rolls). This engine grounds every calculation in realistic hit probability distributions, saving throw failure rates, standard deviation variance analysis, and Armor Class sensitivity curves.

# **Section 2: Ruleset Priority & Dual-Sandbox Architecture**

## **Strict Sandbox Isolation**

To maintain strict mechanical accuracy, selecting a ruleset locks the entire evaluation engine into that specific edition's rule parameters. Cross-edition rule pollution—such as applying 2024 Weapon Masteries to 2014 power-attack feats—is explicitly prohibited by hard boundaries in the rule-parsing layer.

```
       +-------------------------------------------------------+
       |                  USER CHOICE / TOGGLE                 |
       +---------------------------+---------------------------+
                                   |
                 +-----------------+-----------------+
                 |                                   |
                 v                                   v
+---------------------------------+ +---------------------------------+
|   2024 REVISED RULESET (P0)     | |    2014 LEGACY RULESET (P1)     |
|       [PRIMARY CORE FOCUS]      | |       [ISOLATED SANDBOX]        |
|---------------------------------| |---------------------------------|
| * Weapon Mastery Chains         | | * Power-Attack Meta (-5/+10)    |
| * Origin & General Feats        | | * Elven Accuracy Triple-Adv     |
| * 2024 Action Economy Rules     | | * Legacy Action Surge Spells    |
| * Revised Spell & Class Chassis | | * Free-Action Divine Smites   |
+---------------------------------+ +---------------------------------+
```

## **Priority 1 (P0): 2024 Revised Ruleset (Core Focus)**

* **Weapon Mastery Chains:** Complete programmatic modeling of weapon-swapping routines within a single attack action, correctly accounting for mastery triggers:  
  * *Nick:* Off-hand attack integrated into the Main Attack action, freeing the Bonus Action.  
  * *Vex:* Advantage loops across sequential attacks against the same target.  
  * *Graze:* Guaranteed floor damage equal to ability modifier on missed attack rolls.  
  * *Topple, Cleave, Push, Sap, and Slow:* Positional, action-denial, and multi-target flow modeling.  
* **Origin Feats & Reworked General Feats:** Direct integration of background-granted Origin Feats alongside rebalanced General Feats (e.g., reworked Great Weapon Master scaling via Proficiency Bonus without attack penalties, updated Sharpshooter, Dual Wielder, Charger, and Mage Slayer).  
* **2024 Action Economy Bottlenecks:** Enforces modern action constraints:  
  * Divine Smite and related spell-smites reclassified as Bonus Actions and spell slot consumption.  
  * Action Surge explicitly restricting the Magic action.  
  * One-spell-slot-per-turn limits rigorously enforced across main actions, bonus actions, and reactions.  
* **Revised Class, Species & Spell Chassis:** Incorporates updated 2024 subclass availability at Level 3, standardized origin traits, revised spell scales, and rebalanced class features.

## **Priority 2 (P1): 2014 Legacy Ruleset (Secondary Sandbox)**

* **Power-Attack Meta:** Algorithmic calculation of the optimal threshold for taking the \-5 penalty to gain \+10 flat damage via classic Sharpshooter and Great Weapon Master feats against target AC curves.  
* **Crit-Fishing & Advantage Stacking:** Support for Elven Accuracy (triple-advantage) calculations paired with expanded critical hit ranges (e.g., Champion Fighter, Hexblade's Curse).  
* **Legacy Bonus-Action & Nova Loops:** Models legacy stacking interactions, such as Polearm Master \+ Great Weapon Master bonus attacks, Crossbow Expert hand-crossbow bonus attacks, double spellcasting via 2014 Action Surge, and free-action Divine Smites applied to critical hits.

# **Section 3: Functional System Architecture**

## **3.1 The Input Layer (Player Intent, Constraints & Encounter Baselines)**

| Input Category | Configurable Options & Behavior |
| :---- | :---- |
| Ruleset Environment | Strict toggle between 2024 Revised (Default / P0 Focus) and 2014 Legacy (P1 Sandbox). |
| Target Archetype / Role | Selectable focus: Single-Target Nova (Rounds 1–2 burst), Sustained DPR (resource-light full day output), AoE / Crowd Control, Frontline Tank / Mitigation, or Hybrid / Support. |
| Target Level | Single level snapshot evaluation (Levels 1–20) or complete level-by-level progression curve modeling. |
| Multiclassing Rules | Toggle On/Off; set maximum allowed classes (e.g., max 2 or 3\) and minimum level commitment per class. |
| Class, Subclass & Species Filters | Full lock (e.g., "Must be a Paladin"), custom whitelist/blacklist, or unconstrained for open solver discovery. |
| Ability Score & Equipment Baseline | Standard Array, Point Buy (27 points), or Custom Inputs; toggles for starting gear vs. magic item access. |
| Optimization Strategy Mode | Toggle between Peak Target-Level Power (max output at specified level) and Smooth Leveling Curve (no weak levels). |
| Encounter Assumptions | Configurable target AC, saving throw bonus profiles, target count, combat length, and rest frequencies. |

## **3.2 The Codified Rules Engine**

### **Structured 5e.tools Corpus Integration**

Every mechanical asset within the 5e.tools dataset is parsed into a structured rule graph. Features are converted from natural language text into executable mathematical triggers:

```
[Target: Attack Roll] -> (Condition: Weapon Has 'Vex') -> (On Hit) -> [Apply State: Advantage on Next Attack]
```

### **Action Economy Arbitration**

The engine prevents illegal character actions by simulating a strict turn state machine:

* **Concentration Tracking:** Prevents simultaneous maintenance of competing Concentration spells (e.g., flagging Hex vs. Hunter's Mark vs. Spirit Guardians conflicts).  
* **Action & Bonus Action Collisions:** Tracks competing demands for Bonus Actions (e.g., 2024 Divine Smite vs. Telekinetic Shove vs. Off-hand attacks).  
* **Object Interactions & Weapon Drawing:** Models weapon stow/draw limits required to execute multi-weapon mastery loops in a single turn.

### **Trigger & Condition Awareness**

Evaluates conditional damage riders and off-turn triggers, including Sneak Attack availability, Colossus Slayer, Booming Blade secondary damage triggers, Great Weapon Master cleave conditions, and War Caster opportunity attacks.

## **3.3 The Output Package (The Actionable Build Dossier)**

| Output Section | Deliverable & Tactical Breakdown |
| :---- | :---- |
| Build Blueprint | Final ability score allocation, species traits, origin/general feat selections, level-by-level class order, and optimal spell preparation lists. |
| Loadout & Gear | Optimized gear layout, armor type, and explicit weapon-swapping sequences (e.g., "Attack 1 with Vex Dagger, attack 2 with Nick Scimitar"). |
| Turn-by-Turn Combat Workflow | Programmatic combat script: Pre-Combat Setup \-\> Round 1 Nova Opener \-\> Round 2+ Sustained Rotation \-\> Reaction Priority Tree. |
| Math & Probability Breakdown | Visual dashboards and interactive graphs displaying expected DPR distributions, hit and critical hit probability breakdowns, saving throw failure rates, variance/reliability bands, and resource cost efficiency curves. |
| AC & Save Sensitivity Curve | Multi-line performance graphs plotting expected damage and control reliability across varying target ACs and saving throw bonuses to highlight structural breakpoints and drop-offs. |

# **Section 4: Key Design Nuances & Solver Differentiators**

## **Leveling Viability vs. "Level 20 Snapshots"**

A build that produces impressive theoretical numbers at Level 20 but performs poorly from Levels 1 through 10 is fundamentally flawed for practical play. The engine includes a **Smooth Leveling Curve** evaluation mode. When enabled, the solver penalizes builds that experience severe power troughs, prioritizing choices that deliver steady, playable progression across every tier of play.

## **Variance vs. Reliability Metrics**

Two distinct character builds may both calculate an average output of 45 DPR:

1. **Build A (High Reliability):** Achieves 45 DPR via an 85% hit rate, low-variance dice, and miss-damage floors (e.g., Graze mastery).  
2. **Build B (High Variance):** Achieves 45 DPR via a 40% hit rate paired with massive swingy critical-hit multipliers.

The solver explicitly calculates and displays standard deviation, variance spread, and reliability scores alongside average DPR, enabling players to choose builds that match their risk tolerance.

```
BUILD A [High Reliability]:  |==========[  Mean: 45 DPR  ]==========|  (Low Variance, High Hit %)
BUILD B [High Variance]:     |-----===  Mean: 45 DPR  ===-----|        (High Variance, Low Hit %)
```

## **Custom Encounter Overrides**

Users can step outside standard target baselines by adjusting specific encounter parameters:

* Enabling **Magic Resistance** on targets (calculating adv/disadv on saving throws).  
* Applying specific elemental damage resistances or immunities (e.g., Fire/Poison resistance).  
* Configuring custom AC and saving throw offsets to simulate specific campaign boss encounters.

# **Section 5: Minimum Viable Product (MVP) Specification**

## **MVP Core Thesis**

The MVP is a focused vertical slice designed to prove one core thesis: an automated, rule-strict 2024 solver paired with visual probability analytics reveals stronger builds—and clearer tactical trade-offs—than static guides. Rather than attempting to digitize every edge-case spell on day one, the MVP narrows its scope to the mechanics where 2024 optimization shines brightest, delivering an interactive analytical dashboard from end to end.

## **5.1 Tight Scope Boundaries (In vs. Out)**

| In Scope — Vertical Slice | Explicitly Out of Scope — Post-MVP |
| :---- | :---- |
| 2024 Revised Ruleset Only: Pure focus on 2024 rules, natively modeling Weapon Masteries, Origin Feats, reworked General Feats, and updated Bonus Action / spell-slot limits. | The 2014 Legacy Ruleset sandbox. |
| Objective Single-Target Damage: Two clear optimization modes—Sustained DPR (resource-light output across multiple rounds) and Single-Target Nova (maximum burst in Rounds 1–2). | Subjective multi-target AoE, crowd-control, and tank-mitigation scoring weights. |
| Martial, Half-Caster & Striker Roster: Fighter, Rogue, Barbarian, Ranger, Paladin, and Monk, paired with self-buffing damage riders (Hunter's Mark, Hex, Divine Smite, Spirit Shroud). | Full-caster control spellbooks, summons, and complex magic item inventories. |
| Standardized Baselines: 27-point buy for ability scores and standard non-magical weapons/armor, with simple toggles for \+1, \+2, or \+3 weapon enhancements. | — |
| Two Progression Lenses: Optimize for Peak Target-Level Power (maximum output at a chosen level) or a Smooth Leveling Curve (penalizing builds that feel weak during early tiers). | — |

## **5.2 Core Solver Guarantees**

* **Illegal-State Prevention:** Automatically blocks conflicting Concentration spells, enforces the one-spell-slot-per-turn rule, and resolves Bonus Action bottlenecks (such as 2024 Divine Smite competing with off-hand attacks).  
* **Weapon Mastery & Swapping Logic:** Models legal weapon draw/stow sequences within a turn so multi-weapon chains—like opening with Vex to generate advantage before triggering a Nick extra attack—are accurately simulated.  
* **Probability-First Scoring:** Evaluates every option against realistic hit probabilities, critical hit ranges, advantage/disadvantage states, and guaranteed miss-damage floors like Graze.

## **5.3 The MVP Dashboard Experience**

Rather than a step-by-step character creation wizard, the MVP dashboard functions as a live interactive tactical sandbox where adjusting constraints updates recommended builds and visual analytics together:

* **Constraint & "Enemy Dummy" Controls:** Set target level (1–20), toggle Sustained vs. Nova, lock or whitelist classes/species, enable or restrict multiclassing, and configure the target dummy with an AC slider (10–25), combat round count, and basic toggles for damage resistance or permanent advantage/disadvantage.  
* **Top-3 Build Comparison:** Displays the \#1 solved build alongside the \#2 and \#3 runner-up builds (showing species, background/origin feat, starting stats, level-by-level class order, feat picks, and weapon loadout) to immediately expose why a choice won (e.g., trading ceiling DPR for higher hit consistency).  
* **Turn-by-Turn Combat Script:** Breaks the winning build's gameplay into Pre-Combat Setup, Round 1 Opener, and Rounds 2+ Sustained Loop, explicitly labeling every Action, Bonus Action, and weapon draw/stow step required to achieve the calculated numbers.  
* **Visual Probability & Analytics Suite:**  
  * **AC Sensitivity Curve:** Multi-line graph plotting expected DPR across target ACs from 10 to 25 for the top three builds—visually revealing where high-accuracy builds or Graze miss-damage overtake swingy heavy hitters.  
  * **DPR Distribution & Reliability Band:** Visual spread showing average DPR alongside hit probability, crit probability, minimum floor damage, and maximum burst ceiling so variance is obvious at a glance.  
  * **Damage Source Attribution:** Proportional breakdown showing how much total damage comes from base weapon strikes, mastery triggers, feat bonuses, and class/spell riders.

## **5.4 MVP Validation Criteria**

The MVP succeeds when a user can lock in a constrained prompt (e.g., "Level 8, Sustained DPR, no multiclassing, against AC 17") and immediately see:

* 1\. A legal, optimal 2024 build with an exact weapon-swapping and action rotation.  
* 2\. Two competitive runner-up builds that expose concrete mathematical trade-offs.  
* 3\. Clear interactive graphs showing where the build dominates and where high target AC or variance causes performance drop-offs.