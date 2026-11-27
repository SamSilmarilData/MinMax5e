# 04: Rules-Engine Architecture, the Turn State Machine and Edition Isolation

> Status: research note (input to ADRs, not a decision). Date: 2026-11-26.
> Conventions: every factual claim carries a link to a primary source (repo file or official doc). **[unverified]** marks claims I could not confirm from a primary source. **[estimate]** marks my own numbers. **[design]** marks my own recommendations.
> Related: `07-tech-stack-and-architecture.md` (TypeScript engine, TypeBox-validated Rules Packs, two-tier Screen/Refine). This note uses its glossary terms (Ruleset, Rules Pack, Effect DSL, Build, Rotation, Screen, Refine) where they apply.

## Question

How should MinMax5e encode 5e rules as executable logic, so that a solver can enumerate and score builds? This covers:

- the representation of features, feats, spells and items;
- an event/trigger pipeline and a modifier-stacking model;
- how build choices are represented so the solver can enumerate them;
- an action-economy turn state machine that guarantees legal turn plans;
- isolation between the 2024 and 2014 rules;
- whether game prose can be turned into mechanics automatically;
- how to test all of this.

## TL;DR

1. **Use a hybrid representation [design].** Most content should be *data* written in a small, closed Effect DSL: modifiers, riders on events, resource costs, and choices. Each DSL primitive is implemented once in TypeScript. A few content items will need behaviour the primitives can't express. For those, a typed **escape hatch** lets the data call a hook registered by id. Every mature system surveyed converges on a version of this split:
   - PF2e's JSON rule elements are each backed by a TS class with lifecycle hooks ([rule-element dir](https://github.com/foundryvtt/pf2e/tree/v14-dev/src/module/rules/rule-element), [base.ts](https://github.com/foundryvtt/pf2e/blob/v14-dev/src/module/rules/rule-element/base.ts)).
   - Forge's card scripts call a shared effect library ([lightning_bolt.txt](https://github.com/Card-Forge/forge/blob/master/forge-gui/res/cardsfolder/l/lightning_bolt.txt)).
   - XMage's per-card Java classes compose reusable `Effect` objects ([LightningBolt.java](https://github.com/magefree/mage/blob/master/Mage.Sets/src/mage/cards/l/LightningBolt.java)).
   - dnd5e's declarative Active Effects ([AE guide](https://github.com/foundryvtt/dnd5e/wiki/Active-Effect-Guide)) need midi-qol's macro hooks for anything conditional ([MACROS.md](https://github.com/tposney/midi-qol/blob/dnd6/MACROS.md)).
2. **Use event-scoped riders and a PF2e-style predicate vocabulary.** Conditions should test a set of facts about the current roll and state, such as `weapon:property:finesse` or `attack:advantage`. PF2e's `Predicate` works exactly this way: string facts plus `and`/`or`/`not`/comparison objects ([predication.ts](https://github.com/foundryvtt/pf2e/blob/v14-dev/src/module/system/predication.ts)). 5e stacking is much simpler than PF2e's typed bonuses. MinMax5e needs only these rules:
   - advantage collapses to a tri-state;
   - same-named effects don't combine;
   - Upgrade (minimum-value) and Set operations;
   - otherwise everything adds ([XPHB rules text](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/book/book-xphb.json), [Advantage](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/variantrules.json)).
3. **Make choices first-class, enumerable domains in data,** never UI prompts. The solver enumerates them, and a Build is a vector of choice assignments. This contrasts with PF2e `ChoiceSet`, which prompts and then stores the answer in a flag ([quickstart](https://github.com/foundryvtt/pf2e/wiki/Quickstart-guide-for-rule-elements)).
4. **Make turns legal by construction, validated by replay, through one shared transition function [design].**
   - A generator produces turn plans using only `legalSteps(state)`.
   - A validator replays any plan through `apply(state, step)`, whether it is generated, hand-written or user-edited.
   - Both use the same function, so there is one source of truth, and a property test asserts that every generated plan validates.
   - The resource set comes straight from the 2024 text:
     - Action, Bonus Action and Reaction;
     - Attack-action attack slots;
     - one equip/unequip per attack ([Attack action](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/actions.json));
     - one spell slot per turn ([XPHB](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/book/book-xphb.json));
     - per-feature once-per-turn flags;
     - concentration;
     - hands and held weapons.
5. **Isolate editions with namespaced Rules Packs over an edition-agnostic kernel.** This agrees with 07's ADR 4. Use 5e.tools content keys (`Name|SOURCE`, e.g. `Dagger|XPHB`) as stable identities. 5e.tools itself keeps both editions side by side, separated by `source` (`PHB` vs `XPHB`), an `edition` field (`classic` / `one`) and `reprintedAs` links ([items-base.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json), [class-rogue.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/class/class-rogue.json)). Foundry dnd5e is the counter-example. It uses a world setting `rulesVersion: modern|legacy` plus `isLegacy` branches scattered through the code ([settings.mjs](https://github.com/foundryvtt/dnd5e/blob/6.1.x/module/settings.mjs), [attack-data.mjs](https://github.com/foundryvtt/dnd5e/blob/6.1.x/module/data/activity/attack-data.mjs)), which is the pollution pattern the spec forbids.
6. **Do not parse prose into triggers at runtime.** The spec's §3.2 implies it does, but the evidence is against it:
   - 5e.tools feature text is prose with inline tags, not structure ([Sneak Attack JSON](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/class/class-rogue.json)).
   - The best industrial example, MTG Arena's purpose-built parser over Magic's heavily templated text, reaches "80% or so" of new cards, and humans finish the rest ([Wizards dev blog](https://magic.wizards.com/en/news/mtg-arena/on-whiteboards-naps-and-living-breakthrough)).

   Instead, **import the structured 5e.tools fields automatically** (weapon properties, masteries, spell level, casting time, concentration, prerequisites). **Hand-author the Effect DSL** for behaviour. An LLM may draft encodings, but a human must review each one and it must pass golden tests. Track a coverage report keyed by content key.
7. **Testing** has four layers:
   - golden DPR tests against hand-computed and spreadsheet values ([prior art: DAndDSimulator](https://github.com/AClaybrook/DAndDSimulator/blob/master/tests/test_reference_values.py));
   - exact-vs-Monte-Carlo agreement tests ([test_exact.py](https://github.com/AClaybrook/DAndDSimulator/blob/master/tests/test_exact.py));
   - property-based metamorphic tests with fast-check ([repo](https://github.com/dubzzz/fast-check));
   - a per-feature rules regression suite, with one scenario per sentence of rules text plus a named **Ruling** for each ambiguity.
8. **Spec correction:** the spec's example "Attack 1 with Vex Dagger" is wrong for 2024. The 2024 Dagger's mastery is **Nick**. The light Vex weapons are Shortsword and Handaxe, and the Hand Crossbow is ranged ([items-base.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json)). The canonical 2024 dual-wield chain is Shortsword (Vex) followed by Scimitar or Dagger (Nick).

---

## 1. Prior-art survey

### 1.1 Foundry VTT: PF2e system ("Rule Elements")

- **Shape.** A rule element is a JSON object on an item. Its `key` selects a type, such as `FlatModifier`, `AdjustModifier`, `DamageDice`, `RollOption`, `ChoiceSet`, `GrantItem`, `ActiveEffectLike` or `Note` ([quickstart](https://github.com/foundryvtt/pf2e/wiki/Quickstart-guide-for-rule-elements)). The source tree has about 40 rule-element implementations, among them `roll-option`, `choice-set`, `damage-dice`, `adjust-degree-of-success`, `roll-twice`, `substitute-roll`, `multiple-attack-penalty` and `strike` ([dir listing](https://github.com/foundryvtt/pf2e/tree/v14-dev/src/module/rules/rule-element)). Example from the quickstart: `{ "key": "FlatModifier", "selector": "athletics", "type": "item", "value": 2 }`.
- **Selectors** name the statistic or roll a rule applies to, such as `athletics`, `strike-damage`, `saving-throw` or `{item|_id}-damage`. Selectors can be arrays ([quickstart](https://github.com/foundryvtt/pf2e/wiki/Quickstart-guide-for-rule-elements)).
- **Predicates and roll options.** A `Predicate` is an array of statements that must all hold. A statement is either a string tested for membership in a set of roll options, or a logic object (`and`/`or`/`nor`/`nand`/`not`, `eq`/`gt`/`gte`/`lt`/`lte`). Numeric comparisons match option strings by prefix, so `{ "gt": ["actor:level", 5] }` matches `actor:level:6` ([predication.ts](https://github.com/foundryvtt/pf2e/blob/v14-dev/src/module/system/predication.ts)). The doc comment notes that the options are "often based on traits, but that is not required. Sneak attack could be an option that is not a trait" (same file).
- **Lifecycle.** Each rule element has a `priority` (default 100) and optional hooks: `beforePrepareData`, `afterPrepareData`, `beforeRoll`, `afterRoll`, `preCreate`, `onCreate`, and `onUpdateEncounter` for encounter events such as the start of a turn ([base.ts](https://github.com/foundryvtt/pf2e/blob/v14-dev/src/module/rules/rule-element/base.ts)). The JSON is therefore a *configuration of code*, not a free-standing DSL.
- **Stacking.** Modifiers have a `type`. Same-type bonuses don't stack except `untyped`, and bonuses and penalties are resolved separately. Of the ability modifiers, only the best one applies ([modifiers.ts](https://github.com/foundryvtt/pf2e/blob/v14-dev/src/module/actor/modifiers.ts), `applyStackingRules`).
- **Choices.** `ChoiceSet` prompts the user and stores the result at `flags.system.rulesSelections.<flag>`. Its options can be filtered by predicates ([quickstart](https://github.com/foundryvtt/pf2e/wiki/Quickstart-guide-for-rule-elements)).
- **Documented pain points** (all from the [quickstart](https://github.com/foundryvtt/pf2e/wiki/Quickstart-guide-for-rule-elements)):
  - string and number coercion (`"2" + "2"` gives `"22"`);
  - whitespace-sensitive formula paths (`@actor.level -2`);
  - values computed late (e.g. `speed.total`) can't be modified by later rule elements, so authors must manage `phase` and `priority`;
  - several FlatModifiers on the same selector need unique `slug`s or some are silently dropped;
  - `ChoiceSet` can't be used on consumables;
  - some advanced rule elements "can damage actors or hang Foundry".
- **Lesson [design]:** the predicate and roll-option idea is excellent: serialisable, testable and composable. Phase and priority ordering is the main source of authoring bugs, so MinMax5e should use a fixed pipeline of named stages instead of numeric priorities.

### 1.2 Foundry VTT: dnd5e system (Active Effects, Activities, Advancement) and midi-qol

- **Active Effects** are path-based data patches. A change targets a data path such as `system.rolls.attack.mwak.bonus` with a mode (Add, Subtract, Multiply, Override, Upgrade, Downgrade). dnd5e does not use the Custom mode. Roll mode is encoded as 1/0/−1, and Upgrade/Downgrade on it block advantage or disadvantage ([Active Effect Guide](https://github.com/foundryvtt/dnd5e/wiki/Active-Effect-Guide)). The guide does not document conditional effects or effects that trigger on events (same page).
- **Activities**, introduced in dnd5e 4.x, model what an item *does*. The types are attack, cast, check, damage, enchant, forward, heal, order, save, summon, teleport, transform and utility ([activity dir](https://github.com/foundryvtt/dnd5e/tree/6.1.x/module/data/activity)).
- **Advancement** models level-up choices with these types: ability-score-improvement, item-choice, item-grant, scale-value, subclass, trait, size and others ([advancement dir](https://github.com/foundryvtt/dnd5e/tree/6.1.x/module/data/advancement)). A `scale-value` table is the right shape for "Sneak Attack dice by level".
- **Conditional riders live in a module.** midi-qol runs a workflow state machine. Every state fires `midi-qol.pre<State>`/`post<State>` hooks (e.g. `preAttackRoll`, `preDamageRoll`, `postActiveEffects`), and items attach `[pass]MacroName` on-use macros (JavaScript) to those passes ([MACROS.md](https://github.com/tposney/midi-qol/blob/dnd6/MACROS.md)). Conditional advantage and other effects are expressed as `flags.midi-qol.*` keys, e.g. `flags.midi-qol.advantage.attack.all` ([README](https://github.com/tposney/midi-qol/blob/dnd6/README.md)).
- **Lesson [design]:** path-patching data covers static numbers well but runs out of expressiveness at triggers. The community's answer was arbitrary user JavaScript on workflow hooks. MinMax5e needs those hooks to be *engine-owned and finite*, so the solver can reason about them (count them, cache them, test them).

### 1.3 Avrae (Discord bot) automation engine

- **Shape:** an automation *tree* of typed nodes ([Automation reference](https://avrae.readthedocs.io/en/latest/automation_ref.html)):
  - `target`;
  - `attack`, with `hit`/`miss` child lists;
  - `save`, with `success`/`fail`;
  - `damage`, `roll`, `variable`;
  - `condition`, with `onTrue`/`onFalse`;
  - `counter`, which spends charges or slots;
  - `ieffect2`, which attaches a tracked effect;
  - `text`.

  Later nodes can read results such as `lastAttackDidHit` and `lastAttackDidCrit`. Conditions are `IntExpression` strings.
- **Passive effects** on an `ieffect2` form a fixed vocabulary: `attack_advantage` (1, −1, or 2 for Elven Accuracy), `to_hit_bonus`, `damage_bonus`, `ac_bonus`, `ac_value` (highest wins), resistances, `save_adv` and others. An effect can also add `attacks` and `buttons` to a combatant's turn (same page).
- **Lesson:** Avrae is the closest public example of 5e *actions* as data. Its hit/miss/crit branching maps directly onto a probability tree. Its passive-effect vocabulary is flat, though. **[unverified]** I found no documented on-hit hook for passive riders such as Sneak Attack firing once per turn on any qualifying hit. Such riders appear to be applied by players or aliases. The FIREBALL dataset (ACL 2023) shows that LLMs can produce *executable Avrae commands* from game context, "particularly after finetuning" ([arXiv 2305.01528](https://arxiv.org/abs/2305.01528)). That is evidence about generating *invocations*, not rule encodings.

### 1.4 Character builders: PCGen, Aurora, Fight Club 5e, Hero Lab, Pathbuilder, Roll20

- **PCGen (LST).** Tags are pipe-delimited text, e.g. `BONUS:<category>|<group>|<formula>`, optionally followed by `PRExxx` prerequisites and `TYPE=`. Bonuses come in four flavours: typeless, typed (`TYPE=name`), `.REPLACE` and `.STACK`, and the type controls stacking. `LIST` substitutes a `CHOOSE` selection ([globalfilesbonus.html](https://github.com/PCGen/pcgen/blob/master/docs/listfilepages/globalfilestagpages/globalfilesbonus.html)).
- **Aurora Builder (XML elements).** Each `<element>` has `<rules>` containing `<grant>`, `<select>` (with `supports=` filters and `number=`) and `<stat name value bonus level requirements>`. The community 2024 Rogue encodes Sneak Attack as **display-only stats** (`sneak-attack:count` by level, `sneak-attack:die`). Its triggering conditions (advantage or an adjacent ally, a finesse or ranged weapon, once per turn) exist only as prose in a `<sheet><description>` ([class-rogue.xml](https://github.com/AuroraLegacy/elements/blob/master/core/players-handbook-2024/classes/class-rogue.xml)). Builders compute the sheet, not the combat.
- **Fight Club 5e XML.** The schema allows only `<modifier category="bonus|ability score|saving throw|skills|…">` over a regex-limited vocabulary, such as `Melee Attacks +1`, plus an `<attack>` string `Name|tohit|damage` ([compendium.xsd](https://github.com/kinkofer/FightClub5eXML/blob/main/Utilities/compendium.xsd)). There are no triggers.
- **Hero Lab (Authoring Kit).** Imperative "eval scripts" run at a phase and a numeric priority. Community material describes getting the timing wrong as the main pitfall, and authors use before/after tags to express dependencies ([wolflair forum thread](https://forums.wolflair.com/threads/evaluation-timing-confusion.64890), [HL 5e colab wiki](https://github.com/Sklore/HL_DD_5e_Colab/wiki/4.-Common-Scripts)). **[unverified]** I could not reach the official kit docs.
- **Pathbuilder 2e.** Closed source. Third-party publishers ship `.json` packs for import ([Demiplane forum](https://forums.demiplane.com/t/pathbuilder-json-import-for-3rd-party-conetent/3293)). **[unverified]** I could not find a public schema for how it encodes effects.
- **Roll20 sheet workers.** The 5e sheet uses imperative JavaScript reacting to attribute changes (`on("change:globalsavemod", …)`), with `getAttrs`/`setAttrs`. There are 40 change handlers in the legacy 5e sheet ([5th Edition Legacy.html](https://github.com/Roll20/roll20-character-sheets/blob/master/DD5thEditionLegacy/5th%20Edition%20Legacy.html)). The model is spreadsheet-style recalculation, with no turn or action-economy concept. The wiki page (https://wiki.roll20.net/Sheet_Worker_Scripts) returned 403, so this is from source only.

### 1.5 Open-source 5e DPR calculators and simulators

- **wolfwood/dndamage (Rust).** Hand-coded structs `Damage`, `Attack`, `Turn` and `HuntersMark` with `Add` implementations and an `ExpectedDamage` trait. About 25 unit tests. Code-as-rules with no data layer ([main.rs](https://github.com/wolfwood/dndamage/blob/main/src/main.rs)).
- **dwwaite/dpr_simulator (Rust).** A Monte Carlo CLI. The user gives one to-hit value and one damage string *per attack*, explicitly to avoid modelling features: "It's not practical to capture all of these without massive bloat" ([README](https://github.com/dwwaite/dpr_simulator)).
- **AClaybrook/DAndDSimulator (Python, BG3-flavoured 5e).** Has three evaluators: `computations/analytic.py` for the mean, `exact.py` for the full PMF and `numerical_simulation.py` for Monte Carlo. Tests check that the exact PMF sums to 1, that its mean equals the analytic mean, and that it agrees with a 100k-round simulation within a stated tolerance. That includes "limited resources and once per turn" cases ([test_exact.py](https://github.com/AClaybrook/DAndDSimulator/blob/master/tests/test_exact.py)). A golden test pins a Level 6 Oath of Vengeance Paladin's DPR at every AC from 10 to 25 against an independent community spreadsheet ([test_reference_values.py](https://github.com/AClaybrook/DAndDSimulator/blob/master/tests/test_reference_values.py)).
- **5e.tools data** is the spec's corpus. It has rich *structured* metadata:
  - weapon `property`, `mastery: ["Nick|XPHB"]`, `dmg1`;
  - spell `time: [{unit: "bonus", condition: "…immediately after hitting…"}]`, `duration.concentration`, `damageInflict`;
  - feat `prerequisite` and `ability.choose`.

  But feature *behaviour* is prose with `{@tag}` markup ([items-base.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json), [spells-xphb.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/spells/spells-xphb.json), [feats.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/feats.json)).

### 1.6 Outside 5e: Magic: The Gathering engines

These are the largest public examples of "rules as executable logic":

- **Forge:** a per-card text DSL calling shared effects, e.g. `A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3` ([lightning_bolt.txt](https://github.com/Card-Forge/forge/blob/master/forge-gui/res/cardsfolder/l/lightning_bolt.txt)).
- **XMage:** one Java class per card composing library effects, e.g. `addEffect(new DamageTargetEffect(3))` ([LightningBolt.java](https://github.com/magefree/mage/blob/master/Mage.Sets/src/mage/cards/l/LightningBolt.java)).
- **MTG Arena:** a C++ engine whose card logic is CLIPS rules. A Python "Game Rules Parser" converts English card text to CLIPS, so that "80% or so of newly written Magic cards … just work". The rest need engineering ([Wizards dev blog, 2023-07-31](https://magic.wizards.com/en/news/mtg-arena/on-whiteboards-naps-and-living-breakthrough)).

### 1.7 Comparison table

| System | Representation | Conditions | Triggers / timing | Stacking | Choices | Edition handling | Fit for a DPR solver |
|---|---|---|---|---|---|---|---|
| Foundry PF2e | JSON rule elements → TS classes | Predicates over roll-option strings | Lifecycle hooks + `priority`/phase | Typed bonuses, highest-of-type | `ChoiceSet` prompt → flag | n/a (PF2e only) | Best conceptual model; priority ordering is fragile |
| Foundry dnd5e | Path-patch Active Effects + Activities + Advancement | None in core AEs | Activities; conditional riders need midi-qol macros | Add/Override/Upgrade/Downgrade | Advancement (item-choice, ASI) | Global `rulesVersion` + `isLegacy` branches + per-item `source.rules` | Good static model; triggers are delegated to user JS |
| midi-qol | Workflow state machine + JS macros + `flags.midi-qol.*` | Arbitrary JS | `pre/post<State>` hooks | Flags | n/a | dnd5e's | Shows the needed hook points; arbitrary code is opaque to a solver |
| Avrae | JSON automation tree | `IntExpression` | Tree branches (hit/miss/save) | Fixed passive vocabulary | `choice` variable | **[unverified]** | Good per-action shape; weak cross-attack riders |
| PCGen | LST tag lines | `PRExxx` | None (static) | `TYPE`, `.STACK`, `.REPLACE` | `CHOOSE` + `LIST` | Separate datasets | Static only |
| Aurora | XML elements (`grant`/`select`/`stat`) | `requirements=` ids | None | `bonus=` grouping | `<select>` | Separate element sets per book | Static only; behaviour stays in prose |
| Fight Club 5e | XML compendium | None | None | Additive | None | Separate files | Display only |
| Hero Lab | Imperative eval scripts | Script | Phase + priority | Script | Script | **[unverified]** | Timing bugs are the documented pain point |
| Roll20 | JS sheet workers | Script | Attribute change events | Script | Sheet UI | Separate sheets | Spreadsheet recalculation, no turns |
| 5e DPR tools | Hand-coded functions, or user-supplied per-attack numbers | Code / none | Code / none | Code | None / manual | 2014 or none | Easy to test; doesn't scale to a corpus |
| Forge / XMage / Arena (MTG) | DSL calling library / class per card / parsed text to CLIPS | DSL / code / rules | Engine event system | n/a | Engine | n/a | Strong evidence for a shared primitive library plus hand finishing |

---

## 2. Design options

### 2.1 Representation of content

| Option | What it looks like | Pros | Cons |
|---|---|---|---|
| **A. Pure declarative DSL** | Every feature is JSON. The engine interprets a fixed vocabulary. | Analysable, so the solver can inspect costs, triggers and once-per-turn flags. Serialisable, diffable and user-extensible. Schema-validated. | Grows into a bad programming language (PF2e's ~40 keys, AE-like path patches, formula strings with coercion bugs ([quickstart](https://github.com/foundryvtt/pf2e/wiki/Quickstart-guide-for-rule-elements))). Odd features (Cunning Strike, Stunning Strike economics) force new primitives. |
| **B. Code as rules** | Each feature is a TS function or plugin, as in XMage or dndamage. | Unlimited expressiveness and the full type system. Easy to debug. | Opaque to the solver: it can't know a feature's costs or which events it listens to without running it. Every feature is code review. Users can't extend it safely. |
| **C. Hybrid [recommended]** | Data in a closed DSL, plus `hook: "id"` escape hatches implemented in the Rules Pack's TS and *declared* with their event, costs and outputs. | 90%+ of content is data **[estimate]**. Hooks are still visible to the solver through their declaration. Matches PF2e, Forge and XMage in practice. | Two authoring modes. The hook interface needs discipline: hooks must be pure functions of (state, event) → effects. |

Authoring cost for the MVP scope [estimate from counts in 5e.tools XPHB data]:

- 166 class-feature entries and 159 subclass-feature entries across the six classes, which have 24 subclasses ([class files](https://github.com/5etools-mirror-3/5etools-src/tree/main/data/class));
- 77 XPHB feats: 43 General, 10 Origin, 12 Epic Boon, 12 Fighting Style ([feats.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/feats.json));
- 38 weapons and 8 masteries ([items-base.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json));
- about 10 to 15 rider spells.

Perhaps 35–45% of these touch single-target damage **[estimate]**. That is roughly 150–200 encodings. If ~85% are pure data at ~15 minutes each including a golden scenario, and the rest are hooks at ~1–2 hours, the total is about **60–100 person-hours [estimate]**. Effects with no DPR impact can carry `effects: []` with `status: "no-dpr-impact"`, so coverage stays honest.

### 2.2 Event / trigger pipeline

These are the fixed, named stages of an attack's resolution. Each one is a hook point; there are no numeric priorities [design]:

```
turn.start
  └─ step chosen (Attack / Magic / BonusAction / Equip / …)
     attack.declare          (choose weapon, target; equip/unequip allowed here per Attack action)
     attack.modifiers        (to-hit adders, advantage sources, crit range)       ← Vex consumption, Bless
     attack.roll             (engine branches: miss | hit | crit; Graze lives on miss)
     attack.hit / attack.crit / attack.miss
     damage.riders           (optional or automatic extra dice)                   ← Sneak Attack, GWM, Hunter's Mark
     damage.afterHit.decide  (decisions taken "immediately after hitting")       ← 2024 Divine Smite (Bonus Action)
     damage.resolve          (crit doubling of dice, resistance)
     attack.after            (state changes for later attacks)                    ← Vex grants pending advantage
turn.end                     (expire "until end of turn", reset once-per-turn flags)
round.boundary               (expire "until end of your next turn")
```

- Riders declare `on`, `predicate`, `limit` (`oncePerTurn`, `oncePerRound`, `usesPerRest`), `cost` (resources), and `optional` (whether the player may decline; optional riders become **Policy** decisions).
- The spec's Booming Blade, Colossus Slayer and War Caster opportunity attacks fit as riders on `attack.hit` or `turn.start` (target-turn) events. Off-turn events (reactions, opportunity attacks) are out of MVP scope but should be reserved names.

### 2.3 Modifier stacking model

The 2024 rules need only these mechanisms:

- **Advantage is a tri-state.** Any number of advantage sources still roll two d20s. Any advantage plus any disadvantage gives neither, "even if multiple circumstances impose Disadvantage and only one grants Advantage" ([XPHB rules text](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/book/book-xphb.json); [Advantage](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/variantrules.json)). Model it as two booleans, `hasAdv` and `hasDis`. 2014 Elven Accuracy is a pack-specific `extraAdvDie` flag; Avrae encodes the same thing as `attack_advantage: 2` ([Avrae](https://avrae.readthedocs.io/en/latest/automation_ref.html)).
- **Same-named effects don't combine.** "The effects of the same spell cast multiple times don't combine. Instead, the most potent effect … applies" ([XPHB](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/book/book-xphb.json)). Every modifier carries a `sourceKey` (content key), and the engine dedupes by `sourceKey`, keeping the maximum.
- **Ops and order.** `base` → `set` (e.g. an item that sets a score) → `upgrade` (take the max, like Foundry's Upgrade mode ([AE guide](https://github.com/foundryvtt/dnd5e/wiki/Active-Effect-Guide))) → `add` → clamp.
- **AC formulas** are *alternatives* (unarmored defence vs armour). Model them as competing `acFormula` candidates with max-select, not as stacking modifiers.
- **Crit dice.** "Roll all of the attack's damage dice twice" ([Critical Hit](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/variantrules.json)). Riders tag their dice as `fromAttack: true` (doubles) or not. Divine Smite says "extra … damage from the attack", so it doubles. This is an interpretation; record it as a Ruling.

### 2.4 Choices the solver enumerates

- A **Choice** node: `{ id, domain, count, predicate?, scope }`.
  - `domain` is a typed query over pack content, e.g. `feats where category="FS"` or `weaponKinds where proficient`.
  - `scope` is one of `build`, `perLongRest` or `perTurn`.
- A Build is a total assignment of all build-scope Choices.
- Per-long-rest choices (2024 Weapon Mastery kinds can change on a Long Rest ([class-rogue.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/class/class-rogue.json))) are optimised per Encounter, just like build choices.
- Per-turn choices (smite or not, Cunning Strike option, which weapon) belong to the **Policy**, not the Build.
- The solver gets the domain sizes for free, which enables dominance pruning. For example, if Fighting Style X never beats Y for any weapon in the loadout, drop X **[design]**.

### 2.5 Turn state machine: legal by construction vs by validation

**Turn state** (per character, per turn) [design, resources taken from the cited rules]:

| Resource / flag | Source |
|---|---|
| `action`, `bonusAction`, `reaction` (1 each) | [Bonus Action](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/variantrules.json): "can't take more than one Bonus Action on a turn" |
| `attackSlots` (from Extra Attack etc.), opened by the Attack action | [Attack action](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/actions.json) |
| `equipToggles`: one equip *or* unequip per attack made as part of the Attack action; two with Dual Wielder's Quick Draw for non-Two-Handed weapons | [Attack action](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/actions.json), [Dual Wielder](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/feats.json) |
| `freeObjectInteraction` (1) | **[unverified]** whether the 2024 rules keep a separate free object interaction alongside the equip rule. The Utilize text treats object interaction as normally happening alongside another activity (paraphrased; [Utilize](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/actions.json)). Record this as a Ruling. |
| `slotSpentThisTurn` (bool) | "On a turn, you can expend only one spell slot to cast a spell" ([XPHB](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/book/book-xphb.json)) |
| `concentration: spellKey \| null` | [Hunter's Mark](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/spells/spells-xphb.json) `concentration: true` |
| `oncePerTurnUsed: Set<featureKey>` | Sneak Attack, Nick ("only once per turn" ([Nick](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json))) |
| `lightExtraAttackAvailable` | Light property: an Attack action with a Light weapon enables one extra attack "later on the same turn" with a *different* Light weapon, using the Bonus Action ([Light](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json)) |
| `hands: [held, held]`, `loadout` | Equip rules above |
| Per-target status: `vexPending(target)` | [Vex](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json) "next attack roll against that creature before the end of your next turn". It crosses the turn boundary, so it is part of the **round** state. |
| `limitedUses` (Paladin's Smite free cast, Action Surge, Channel Divinity) | e.g. [Paladin's Smite](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/class/class-paladin.json) lets you cast Divine Smite "without expending a spell slot" once per Long Rest, so it does **not** set `slotSpentThisTurn` but does use the Bonus Action |

**Options:**

- **Validation only.** Generate freely and reject illegal plans. This is simple, but the search wastes effort and can't explain *why* a plan is illegal unless the validator is rich.
- **By construction only.** The generator encodes the rules, so nothing checks plans a human edits or plans built by hand in tests.
- **[Recommended] Both through one transition function.** `legalSteps(state) → Step[]` and `apply(state, step) → state | Illegal(reason)` live in the kernel, parameterised by the Ruleset's economy rules.
  - The generator is a search over `legalSteps`.
  - The validator replays a plan with `apply` and returns the first `Illegal(reason)`. This becomes the dashboard's explanation, e.g. "Bonus Action already used by Divine Smite".
  - A property test asserts `∀ generated plan: validate(plan) = ok`.
  - A second property test asserts that removing any step either stays legal or yields a reason, never a crash.

**The spec's examples, worked through the state machine:**

1. **Vex → Nick (Fighter 5, Shortsword + Scimitar both held, both kinds mastered).**
   - `Attack` opens 2 slots.
   - Slot 1: Shortsword hits, `attack.after` sets `vexPending(T)`.
   - Slot 2: Shortsword with advantage, which consumes Vex. On a hit, Vex is pending again.
   - The Light extra attack is enabled. Because the Scimitar has Nick and Nick is unused, it is made *as part of the Attack action*, and the Bonus Action stays free. The extra attack adds no ability modifier to damage ([Light](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json)).
   - Which weapon must carry Nick (the first or the extra attack's) is a **Ruling** to record. The text says "When you make the extra attack of the Light property, you can make it as part of the Attack action" ([Nick](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json)).
2. **Divine Smite vs off-hand Bonus Action (Paladin, two Light weapons without Nick).**
   - After a melee hit, `damage.afterHit.decide` offers Divine Smite, which costs a Bonus Action and a slot ([Divine Smite](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/spells/spells-xphb.json)).
   - If the Policy takes it, `bonusAction=0`, and the later Light extra attack is illegal because it needs the Bonus Action. The validator returns that reason.
   - With a Nick weapon, both are legal.
   - Note the timing: the Bonus Action is spent *in the middle of* the Attack action. So the state machine must allow Bonus Action steps at trigger points, not only between actions.
3. **One slot per turn.**
   - Round 1: Hunter's Mark (Bonus Action, slot) sets `slotSpentThisTurn`, so a slotted Divine Smite on that turn is illegal. A Paladin's Smite free cast is also blocked, because the Bonus Action is already spent.
   - Without a Bonus Action spell, the same restriction blocks two slotted spells, e.g. a Magic action spell plus a slotted smite.

**Probabilistic execution [design].** A Turn Plan is a *policy tree*, not a fixed list, because later steps depend on hit/miss/crit outcomes: Vex, once-per-turn Sneak Attack placement, and the decision to smite on a crit. The engine should therefore evaluate an **outcome tree**:

- each attack branches into miss, hit or crit;
- branches with identical turn states are merged (memoised on state);
- damage PMFs are convolved along each branch.

This has two consequences:

- Once-per-turn riders make attacks **dependent**. P(Sneak Attack lands) = 1 − P(no qualifying hit), not a per-attack sum. So 07's "variance of independent sums is additive" Screen shortcut holds only *within* a branch, and the Screen must still branch on rider-relevant state.
- Sustained DPR over N rounds is a Markov chain over round-boundary state (`vexPending`, concentration up or down, remaining slots).

### 2.6 Edition isolation

| Option | Example | Verdict |
|---|---|---|
| Global flag + branches in shared code | dnd5e `rulesVersion` and `isLegacy` checks inside `attack-data.mjs` and `feat.mjs`. Items carry `source.rules` "2014"/"2024", defaulting from the world setting ([settings.mjs](https://github.com/foundryvtt/dnd5e/blob/6.1.x/module/settings.mjs), [attack-data.mjs](https://github.com/foundryvtt/dnd5e/blob/6.1.x/module/data/activity/attack-data.mjs), [feat.mjs](https://github.com/foundryvtt/dnd5e/blob/6.1.x/module/data/item/feat.mjs), [source-field.mjs](https://github.com/foundryvtt/dnd5e/blob/6.1.x/module/data/shared/source-field.mjs)) | Reject: the cross-edition pollution the spec forbids |
| Shared corpus, editions as namespaces | 5e.tools: `PHB` vs `XPHB` sources, `edition: classic/one`, `reprintedAs`, references qualified as `Nick|XPHB` ([items-base.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json)) | Adopt for **content identity** |
| Fully separate engines | Separate codebases | Reject: duplicated dice, PMF and search code that would drift |
| **Edition-agnostic kernel + one Rules Pack per Ruleset [recommended]** | Kernel: PMFs, outcome tree, state-machine *skeleton*, DSL interpreter. Pack: economy rules (2024 one slot per turn; 2014 bonus-action-spell rule), crit rule, stacking quirks, content, hooks, Rulings | Matches 07 ADR 4. Packs can't import each other, and a content key from another pack fails at load time. |

Edition-specific *rules*, not just content, live in the pack. Examples: 2014 Divine Smite as a no-action rider on hit vs the 2024 Bonus Action spell, and 2014 GWM −5/+10 as a per-attack Policy choice. The kernel exposes **extension points** such as the `EconomyRules` interface, so the kernel itself never contains `if (edition === …)` **[design]**.

### 2.7 Natural language → mechanics

- **Evidence against runtime parsing:**
  - 5e.tools features are prose with tags, e.g. "Once per turn, you can deal an extra {@damage 1d6} damage … if you have {@variantrule Advantage|XPHB} …" ([class-rogue.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/class/class-rogue.json)).
  - Aurora's hand-authored 2024 encoding still leaves the trigger in prose ([class-rogue.xml](https://github.com/AuroraLegacy/elements/blob/master/core/players-handbook-2024/classes/class-rogue.xml)).
  - MTG Arena's deterministic parser reaches about 80% on text far more templated than D&D's, and humans "get the other 20% to work" ([Wizards](https://magic.wizards.com/en/news/mtg-arena/on-whiteboards-naps-and-living-breakthrough)).
  - Academic card-text-to-code reached 22.7% exact match on the Hearthstone benchmark in 2017 ([ASN, arXiv 1704.07535](https://arxiv.org/abs/1704.07535); dataset [deepmind/card2code](https://github.com/deepmind/card2code)).
  - **[unverified]** I found no published benchmark of modern LLMs on 5e feature-to-DSL extraction. FIREBALL covers command generation, not rule encoding ([arXiv 2305.01528](https://arxiv.org/abs/2305.01528)).
- **What *is* automatable today:** importing 5e.tools' structured fields, listed in §1.5.
- **Recommendation [design]:**
  1. Import structured fields deterministically.
  2. Hand-author the Effect DSL, optionally with LLM-*drafted* encodings constrained by the TypeBox schema and checked against generated golden scenarios.
  3. A human reviews every encoding and the original text is kept alongside it.
  4. CI produces a **coverage report**: every content key in scope is `encoded`, `no-dpr-impact` or `missing`.
  5. Pilot the LLM-drafting step on about 20 features to measure its acceptance rate before relying on it.

### 2.8 Testing and verification

1. **Golden DPR tests.** Hand-computed expected DPR for small, fully specified scenarios, e.g. "Rogue 5, Shortsword/Scimitar, AC 15, ally adjacent". Also pin selected community spreadsheet curves, as DAndDSimulator does for every AC from 10 to 25 ([test_reference_values.py](https://github.com/AClaybrook/DAndDSimulator/blob/master/tests/test_reference_values.py)). Store expected values as exact rationals where possible.
2. **Three-way oracle.** For the same Build, compare the analytic mean, the exact PMF mean and Monte Carlo (seeded). The PMF must sum to 1, and the means and shape must agree within sampling tolerance ([test_exact.py](https://github.com/AClaybrook/DAndDSimulator/blob/master/tests/test_exact.py)). An independent PMF library such as icepool ([repo](https://github.com/HighDiceRoller/icepool)) can cross-check the Distribution Kernel offline.
3. **Property-based and metamorphic tests** with fast-check ([repo](https://github.com/dubzzz/fast-check)):
   - Raising a weapon's enhancement (+0 → +1) never lowers the *optimal-policy* mean DPR. It *can* lower a fixed policy's result if a threshold choice flips, so test against the optimal policy.
   - Lowering target AC never lowers DPR.
   - Adding advantage never lowers per-attack hit chance.
   - Granting an extra resource (a slot or a use) never lowers optimal Nova DPR.
   - Every generated plan validates (§2.5).
   - The PMF is normalised and non-negative.
   - DPR is invariant to the order in which content is loaded.
   - Edition isolation: loading a 2024 pack plus a reference to `|PHB` content fails.
4. **Rules regression suite.**
   - One scenario test per rules sentence of each encoded feature, e.g. Sneak Attack: (a) advantage plus finesse fires; (b) ally adjacent, no disadvantage, fires; (c) ally adjacent *with* disadvantage doesn't fire; (d) second hit in the same turn doesn't fire; (e) a non-finesse melee weapon doesn't fire.
   - Each ambiguity gets a named **Ruling** with a test pinned to it.
5. **Snapshot tests** on solver output (top-3 Builds plus Rotation) for the spec's validation prompt ("Level 8, Sustained, no multiclass, AC 17"), reviewed whenever they change.

---

## 3. Recommended architecture sketch [design]

```
                 5e.tools JSON (structured fields)        Hand-authored encodings (+ optional LLM drafts, human-reviewed)
                              │                                         │
                              ▼                                         ▼
                 ┌──────────────────────────── Rules Pack (one per Ruleset) ─────────────────────────────┐
                 │ content (keyed "Name|SOURCE")  effects (Effect DSL)  choices  hooks (TS, declared)    │
                 │ EconomyRules  CritRule  StackingRules  Rulings[]  coverage manifest                   │
                 └───────────────────────────────────────┬───────────────────────────────────────────────┘
                                                         │ injected at runtime (kernel never imports a pack)
 ┌───────────────────────────────── Edition-agnostic kernel ──────────────────────────────────────────────┐
 │                                                                                                        │
 │  Build Compiler ──────────► Compiled Character ──────────► Turn Engine ──────────► Outcome Evaluator   │
 │  (choice assignment →       (static stats, modifiers,      legalSteps / apply      (miss/hit/crit tree, │
 │   modifiers, AC formula,     riders indexed by event,      over TurnState;          state-merged;       │
 │   resources)                 resources)                    Policy decides          PMFs via Distribution│
 │       ▲                                                    optional riders)        Kernel; Markov over  │
 │       │                                                         ▲                  rounds)              │
 │  Choice Enumerator ◄── Solver (Screen → Refine) ────────────────┘                        │               │
 │                                                                                          ▼               │
 │                                       Validator (replays any plan via apply → Illegal(reason))           │
 └──────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

### Example encodings (Effect DSL, TypeBox-validated TS literals; pack `rules-2024`)

**1. Vex (weapon mastery property)**

```ts
export const vex = defineMastery({
  key: "Vex|XPHB",
  requires: { masteredWeaponKind: "$weapon.kind" },         // Weapon Mastery choice must include this weapon kind
  riders: [{
    on: "attack.after",
    predicate: ["attack:outcome:hit-or-crit", "attack:dealt-damage", "attack:weapon:mastery:vex"],
    effect: { op: "grantStatus", status: "vexPending", scope: { target: "$target" },
              consumeOn: "attack.modifiers:self->target",   // your next attack roll vs that creature
              expires: "end-of-your-next-turn" },
  }],
  statusEffects: {
    vexPending: [{ on: "attack.modifiers", predicate: ["attack:target:has-status:vexPending:by-self"],
                   effect: { op: "advantage" }, consume: true }],
  },
  text: "If you hit a creature with this weapon and deal damage to the creature, you have Advantage on your next attack roll against that creature before the end of your next turn.",
});
```

**2. Sneak Attack (2024 Rogue)**

```ts
export const sneakAttack = defineFeature({
  key: "Sneak Attack|XPHB",
  scale: { id: "sneakDice", byClassLevel: { class: "Rogue|XPHB", table: [1,1,2,2,3,3,4,4,5,5,6,6,7,7,8,8,9,9,10,10] } },
  riders: [{
    on: "damage.riders",
    limit: "oncePerTurn",
    optional: false,                       // [Ruling R-SA-1: always apply at first eligible hit; revisit with Cunning Strike]
    predicate: [
      "attack:outcome:hit-or-crit", "attack:uses-attack-roll",
      { or: ["weapon:property:finesse", "attack:ranged-weapon"] },
      { or: [
        "attack:advantage",
        { and: ["target:ally-within-5ft:not-incapacitated", { not: "attack:disadvantage" }] },
      ]},
    ],
    effect: { op: "extraDamage", dice: { count: "@scale.sneakDice", die: 6 },
              damageType: "$weapon.damageType", fromAttack: true },   // doubles on crit
  }],
});
```

Note that `attack:advantage` means *net* advantage after the tri-state collapse. Advantage cancelled by disadvantage does not qualify, which matches "you have Advantage on the roll".

**3. Divine Smite (2024 spell) + Paladin's Smite**

```ts
export const divineSmite = defineSpell({
  key: "Divine Smite|XPHB",
  imported: { level: 1, castingTime: "bonus", concentration: false },      // from spells-xphb.json
  trigger: {
    on: "damage.afterHit.decide",                                          // "immediately after hitting"
    predicate: ["attack:outcome:hit-or-crit",
                { or: ["attack:melee-weapon", "attack:unarmed-strike"] }],
    optional: true,                                                        // Policy decides
    cost: { oneOf: [
      { bonusAction: 1, spellSlot: { minLevel: 1, upcast: true } },        // sets slotSpentThisTurn
      { bonusAction: 1, use: "Paladin's Smite|XPHB" },                     // free cast: no slot, still BA
    ]},
    effect: { op: "extraDamage",
              dice: { count: "1 + @slotLevel", die: 8 },                   // 2d8 at 1st, +1d8/level
              bonus: { predicate: [{ or: ["target:type:fiend", "target:type:undead"] }], dice: { count: 1, die: 8 } },
              damageType: "radiant", fromAttack: true },                   // [Ruling R-DS-1: doubles on crit]
  },
});
```

The dice formula follows the spell text: 2d8 at 1st level and +1d8 per slot level above 1st, so `count = 1 + slotLevel` (5e.tools tag `{@scaledamage 2d8|1-9|1d8}`) ([spells-xphb.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/spells/spells-xphb.json)). The free Paladin's Smite cast is at 1st level. **[unverified]** The text says only that you cast it without a slot; that this means 1st level is a Ruling to record.

What the kernel provides so these three stay pure data:

- the stage names;
- the predicate fact vocabulary (`attack:*`, `weapon:*`, `target:*`);
- the ops (`advantage`, `extraDamage`, `grantStatus`, `flatModifier`, `upgrade`, `set`);
- limits, costs and Policy hooks.

None of the three needs a code hook. A feature such as Cunning Strike (trading Sneak Attack dice for effects) would use `hook: "rogue.cunningStrike"` with a declared cost and outputs.

---

## 4. Open questions

1. **Policy search.** For Nova and Sustained objectives, should the engine compute the *optimal* Policy per Build (expectimax over the outcome tree, which is exact for the mean but harder with variance-aware objectives) or choose among a few named policies ("smite on crit only", "smite first hit")? Named policies are easier to explain in the Turn-by-Turn script.
2. **Ruling register.** Who owns Rulings, such as which weapon needs Nick, crit-doubling of smite dice, and whether the 2024 rules keep a separate free object interaction? Should users be able to toggle them, like "house rule" switches?
3. **The "ally within 5 ft" assumption** for Sneak Attack. Should it be an Encounter parameter (probability that an ally is adjacent) or a boolean? A probability makes Sneak Attack a mixture distribution.
4. **Multi-target state** (Vex is per target; Cleave and Topple change targets) is out of the MVP's single-target scope. Should the TurnState data model still be keyed by target id from day one? That is cheap now and expensive later.
5. **Off-turn events** (opportunity attacks, Sneak Attack "once per turn" on someone else's turn, reactions). Reserve the stage names now. Are they needed for the MVP's Sustained DPR at all?
6. **The licensing boundary.** Encodings of non-SRD content, such as Paladin subclasses that aren't in SRD 5.2, might be distributable separately from the text. This is a legal question, not an engineering one; 07 discusses the SRD-only shipping posture.
7. **Predicate typing.** Should facts be raw strings like PF2e's, or a typed union serialised to strings? Typed facts catch typos at pack build time, which is a common PF2e pitfall since silent non-matches look like "feature doesn't work".

---

## Candidate ADRs

Only decisions that are hard to reverse, surprising without context and the result of a real trade-off. Edition isolation as packages is already 07's ADR 4. The proposal below extends it with content keys and pack-owned economy rules rather than duplicating it.

1. **Hybrid Effect DSL: closed declarative vocabulary plus declared code hooks.**
   - **Decision:** content behaviour is data built from a fixed set of kernel primitives (stages, predicates, ops, limits, costs). Where those are insufficient, data references a TS hook by id. Each hook *declares* its stage, costs and output kinds, and must be a pure function of (TurnState, event).
   - **Rationale:** the solver must be able to inspect costs and triggers without running code (enumeration, pruning, legality). Fully declarative systems grow fragile ad-hoc formula languages ([PF2e quickstart pain points](https://github.com/foundryvtt/pf2e/wiki/Quickstart-guide-for-rule-elements)). Fully coded systems are opaque and costly to review ([XMage](https://github.com/magefree/mage/blob/master/Mage.Sets/src/mage/cards/l/LightningBolt.java)).
   - **Rejected:** pure DSL; code-as-rules; arbitrary user-script hooks like midi-qol macros ([MACROS.md](https://github.com/tposney/midi-qol/blob/dnd6/MACROS.md)).
2. **Fixed named resolution stages instead of numeric priorities.**
   - **Decision:** riders attach to a closed list of stages (§2.2). Ordering within a stage is defined by op semantics (set → upgrade → add), never by author-supplied priority numbers.
   - **Rationale:** phase and priority mistakes are the documented top pitfall in PF2e and Hero Lab ([PF2e quickstart](https://github.com/foundryvtt/pf2e/wiki/Quickstart-guide-for-rule-elements), [wolflair thread](https://forums.wolflair.com/threads/evaluation-timing-confusion.64890)).
   - **Surprising because** it forbids a common extensibility knob.
   - **Rejected:** PF2e-style `priority`; Hero Lab phase plus priority.
3. **One transition function for both plan generation and plan validation.**
   - **Decision:** `legalSteps` and `apply` are the single authority on action-economy legality. The generator only follows `legalSteps`, and every plan, including user-edited ones, is replayed through `apply`, which returns typed `Illegal(reason)`s.
   - **Rationale:** gives the guarantees in spec §5.2 together with explainable rejections, and supports a property test that the generator emits only valid plans.
   - **Rejected:** validate-only (wasteful search, weak explanations); construct-only (unverifiable, no path for user edits).
4. **Turn plans are policy trees evaluated over a state-merged outcome tree.**
   - **Decision:** a Turn Plan's later steps may depend on earlier hit/miss/crit outcomes. The evaluator branches per attack, merges identical TurnStates and convolves PMFs along branches. Sustained DPR is a Markov chain over round-boundary state.
   - **Rationale:** Vex, once-per-turn Sneak Attack and post-hit smite decisions make attacks dependent. A flat list of independent attacks gives wrong means, not just wrong variances.
   - **Surprising because** it qualifies 07's "variance is additive" Screen shortcut.
   - **Rejected:** independent-attack sums; Monte Carlo as the primary evaluator (noise, non-determinism; keep it as a test oracle).
5. **Content identity uses 5e.tools `Name|SOURCE` keys, and each Rules Pack owns its edition's economy, crit and stacking rules.**
   - **Decision:** every encoding, choice domain and coverage entry is keyed by the 5e.tools content key, e.g. `Sneak Attack|XPHB`. The kernel contains no edition branches; packs implement `EconomyRules`, `CritRule` and `StackingRules`. A key from another pack is a load-time error.
   - **Rationale:** 5e.tools already separates editions this way ([items-base.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json)), which makes coverage reporting and re-import cheap. dnd5e's `isLegacy` branching is the anti-pattern ([attack-data.mjs](https://github.com/foundryvtt/dnd5e/blob/6.1.x/module/data/activity/attack-data.mjs)).
   - **Rejected:** internal UUIDs (lose the import and coverage join); edition flags in the kernel.
6. **No runtime natural-language parsing; behaviour is human-authored, with coverage enforced in CI.**
   - **Decision:** the spec's "parsed into a structured rule graph" is satisfied by deterministic import of 5e.tools *structured fields* plus human-authored (optionally LLM-drafted, always reviewed) Effect DSL encodings. CI fails if an in-scope content key is `missing`.
   - **Rationale:** feature text is untemplated prose ([class-rogue.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/class/class-rogue.json)). Even MTG Arena's dedicated parser leaves about 20% to humans ([Wizards](https://magic.wizards.com/en/news/mtg-arena/on-whiteboards-naps-and-living-breakthrough)), and a wrong trigger silently corrupts solver rankings.
   - **Surprising because** it departs from the spec's wording.
   - **Rejected:** runtime parsing; unreviewed LLM extraction.
7. **Choices are declarative, enumerable domains; per-turn decisions belong to the Policy, not the Build.**
   - **Decision:** a Choice has a typed domain query, a count and a scope (`build` / `perLongRest` / `perTurn`). Build-scope and per-long-rest Choices are assigned by the solver. Per-turn ones are decided by the Policy during evaluation.
   - **Rationale:** the solver needs domain sizes for enumeration and pruning. Prompt-and-store designs (PF2e `ChoiceSet` ([quickstart](https://github.com/foundryvtt/pf2e/wiki/Quickstart-guide-for-rule-elements))) hide the domain.
   - **Rejected:** UI-prompt choices; treating smite and weapon selection as Build attributes.

## Glossary candidates

| Term | Definition | Synonyms to avoid |
|---|---|---|
| **Content Key** | The stable identity of a content item, in 5e.tools `Name|SOURCE` form (e.g. `Vex|XPHB`). | id, slug, uuid |
| **Encoding** | The hand-authored Effect DSL representation of one content item's behaviour. | script, automation, implementation |
| **Rider** | A conditional effect attached to a resolution Stage, with a predicate, limit, cost and optional flag. | trigger (as a noun), proc, hook |
| **Stage** | One of the fixed, named points in attack and turn resolution (e.g. `damage.riders`) where Riders attach. | phase, priority, event (in code) |
| **Hook** | A pack-provided TS function, referenced by id from an Encoding and declared with its Stage, costs and outputs, used when primitives are insufficient. | macro, plugin, callback |
| **Predicate** | A boolean expression over Facts that gates a Rider or Choice option. | condition, filter, requirement |
| **Fact** | A namespaced string describing the current roll or state (e.g. `weapon:property:finesse`), tested by Predicates. | roll option, tag, trait |
| **Modifier** | A numeric or roll-mode change with an op (`set`, `upgrade`, `add`, `advantage`) and a Content Key used for same-name deduplication. | bonus, buff, effect |
| **Choice** | A declarative decision with a typed domain, count and scope (`build`, `perLongRest`, `perTurn`). | option, pick, selection |
| **Policy** | The rule that makes per-turn decisions during evaluation (e.g. smite only on a crit). | strategy, AI, tactic |
| **Turn State** | The per-turn resources and flags (action, bonus action, attack slots, equip toggles, slot-spent, once-per-turn set, hands) that the transition function operates on. | context, economy, budget |
| **Turn Plan** | One turn of a Rotation, a policy tree of Steps validated by replay. | combo, script, sequence |
| **Step** | One atomic economy transition: take an action, make an attack, equip, cast. | move, op, command |
| **Outcome Tree** | The branching miss/hit/crit evaluation of a Turn Plan, with merged identical states. | simulation, roll tree |
| **Ruling** | A named, test-pinned interpretation of ambiguous rules text, owned by a Rules Pack. | house rule, assumption, errata |
| **Coverage Report** | A CI artifact listing every in-scope Content Key as `encoded`, `no-dpr-impact` or `missing`. | completeness, progress |
