# 01 — Rules data source, data format and licensing risk

*Researched 2026-11-26. This is engineering research, not legal advice. Every legal conclusion below needs review by counsel before anything public ships.*

## Question

The spec names "the complete 5e.tools corpus" as MinMax5e's mechanical data source (spec, Metadata Summary and "Structured 5e.tools Corpus Integration"). This document asks:

1. What does the 5e.tools data actually look like, and how much of the mechanics is machine-readable?
2. Is it legal to build on it, especially for a product with any public or commercial side?
3. What are the alternatives, and how do they compare on coverage, structure, license and 2024 freshness?
4. What data strategy should MinMax5e adopt, and how should 2014 and 2024 content be split at the data layer?

## TL;DR

- **5e.tools data is well structured for metadata and plain prose for mechanics.** Classes, weapons, spells and feats have typed fields (hit die, mastery, damage dice, prerequisites, ability increases). But what a feature *does* lives in `entries` prose, decorated with display tags (`{@dice}`, `{@damage}`, `{@variantrule}`). In the six MVP classes, only about 31 of 325 2024 feature records carry a dice or damage tag, and none encode triggers, conditions or action economy. **An executable rule graph has to be hand-authored no matter which source we pick.**
- **5e.tools is not a licensable source.** The repo's MIT license covers the site code, but the data is mostly verbatim non-SRD WotC text. WotC filed a DMCA notice against 5etools GitHub mirrors in August 2024, saying they consist "almost exclusively of content copied verbatim from Wizards' D&D publications". Shipping, hosting or bundling that corpus is high risk.
- **SRD 5.2.1 (CC-BY-4.0, May 2025) is the only official, commercially usable 2024 source.** It covers all 12 classes, but only **one subclass per class**, **17 feats** (out of the 2024 PHB's roughly 77 feat records), 4 of 16 backgrounds, 9 of 10 species (no Aasimar), all 8 weapon masteries and every PHB weapon.
- **The MVP's most important optimizer content is not in the SRD.** Missing feats include Great Weapon Master, Sharpshooter, Polearm Master, Crossbow Expert, Dual Wielder, Sentinel, Shield Master, Charger, Piercer, Slasher, Crusher, Tough, Lucky, Tavern Brawler, the Dueling fighting style and others. Missing subclasses include Battle Master, Gloom Stalker, Vengeance, Assassin, Zealot and others. Missing spells include Thunderous, Wrathful, Blinding, Staggering and Banishing Smite, Hail of Thorns, Lightning Arrow, Swift Quiver, Conjure Barrage, Steel Wind Strike and Elemental Weapon.
- **The Fan Content Policy does not rescue a non-SRD product.** It requires content to be free (no paywall, subscription or sign-up gate). It can be revoked "for any reason or no reason". It also says fan content cannot incorporate WotC "game mechanics (unless … under the D&D Open Game License)".
- **US copyright does not protect game mechanics, only their written expression.** "The idea for a game is not protected by copyright … the methods for playing it are not protected." Hand-authored *mechanical encodings*, in our own words with no copied WotC text, are therefore a much lower-risk way to cover non-SRD content than ingesting 5e.tools. They are not zero-risk: names, trademarks and contract terms still apply. This needs counsel.
- **Alternatives:** Open5e (`srd-2024` document) and 5e-bits/dnd5eapi (`/api/2024`) both serve SRD 5.2 as clean JSON, but feature mechanics are still prose. The Foundry `dnd5e` system has SRD 5.2 compendia with partial Active Effect automation; its code is MIT and its content CC-BY. The non-SRD 2024 books on Foundry are paid premium modules. D&D Beyond has no public API, and its Terms forbid bots, scrapers and extraction tools.
- **Recommendation:** use SRD 5.2.1 as the shipped baseline, plus a project-authored mechanics layer (the rule graph) as the real product asset, plus an optional local-only "bring your own content" import that never leaves the user's machine. Put the ruleset into every content ID and namespace (`2024` / `2014`) and record provenance (license) on every record.
- **2014 vs 2024 at the data layer:** 5e.tools marks edition with source codes (`PHB` vs `XPHB`) and an `edition: classic|one` hint. It also *synthesizes* cross-edition subclass copies (for example a 2014 XGE Samurai attached to the 2024 Fighter). That violates the spec's no-cross-edition-pollution rule, so any importer must drop those copies.

## Detailed findings

### 1. The 5e.tools data

#### 1.1 Repository and schema

- **Data.** The data lives at [`5etools-mirror-3/5etools-src`](https://github.com/5etools-mirror-3/5etools-src), last pushed 2026-11-23. Its README says "This project is licensed under the terms of the MIT license" ([README](https://github.com/5etools-mirror-3/5etools-src/blob/main/README.md), [LICENSE.md](https://github.com/5etools-mirror-3/5etools-src/blob/main/LICENSE.md)).
- **Layout.** The `data/` folder is split by entity type: `data/class/class-<name>.json`, `data/feats.json`, `data/items-base.json`, `data/spells/spells-<source>.json`, `data/races.json` (species), `data/backgrounds.json` and `data/books.json`. Fluff (flavor text) is kept in separate `fluff-*.json` files ([data/class listing](https://github.com/5etools-mirror-3/5etools-src/tree/main/data/class)).
- **Schema.** JSON Schema (draft 2020-12) lives in [`TheGiddyLimit/5etools-utils/schema/site`](https://github.com/TheGiddyLimit/5etools-utils/tree/main/schema/site): `class/class.json`, `feats.json`, `spells/…`, `items-base.json`, `entry.json` (the recursive `entries` format), `util-edition.json` and others. That repo has no declared license (GitHub API returns `license: null`), which is *unverified/unclear* for reuse of the schema itself.
- **Contributor rule.** CONTRIBUTING says: "Only "official" (that is, published by WotC) data is to be included in the site." Homebrew goes to [`TheGiddyLimit/homebrew`](https://github.com/TheGiddyLimit/homebrew) instead ([CONTRIBUTING.md](https://github.com/5etools-mirror-3/5etools-src/blob/main/CONTRIBUTING.md)).

#### 1.2 Entity structure (observed in the actual files)

**Class** ([class-fighter.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/class/class-fighter.json)). The file holds four arrays: `class`, `subclass`, `classFeature` and `subclassFeature`.

The 2024 Fighter record has these structured fields:
- `hd: {number:1, faces:10}`
- `primaryAbility`
- `featProgression` (Fighting Style at level 1, Epic Boon at level 19)
- `itemMasteries: true` and `itemMasteriesProgression: {"1":3,"4":4,"10":5,"16":6}`
- `classTableGroups` (for example the Second Wind uses column)
- `classFeatures`, as reference strings such as `"Extra Attack|Fighter|XPHB|5"`

**Class features** are records with `name/source/page/className/level/entries`. Their mechanics are prose only. Example, the 2024 Extra Attack:

> `"entries": ["You can attack twice instead of once whenever you take the {@action Attack|XPHB} action on your turn."]`

Champion's Improved Critical:

> `"…can score a {@variantrule Critical Hit|XPHB} on a roll of 19 or 20 on the {@dice d20}."`

**Feat** ([feats.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/feats.json)). Structured fields are `category` (`G`, `O`, `FS`, `EB`), `prerequisite` (for example `{level:4, ability:[{str:13}]}`), `ability` (the ability score increase), `additionalSpells`, proficiencies and `repeatable`. Effects are in `entries`. For example, Great Weapon Master's "Heavy Weapon Mastery" and "Hew" are both prose. Nothing encodes "+PB damage, once per turn, Heavy weapon, Attack action, on your turn".

**Weapon** ([items-base.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/items-base.json)). This is well structured. Example:

> `{"name":"Greatsword","source":"XPHB","weaponCategory":"martial","property":["H|XPHB","2H|XPHB"],"mastery":["Graze|XPHB"],"dmg1":"2d6","dmgType":"S"}`

The 8 mastery properties are a separate `itemMastery` array, but their effects are prose. Graze, for example, reads "…you can deal damage to that creature equal to the ability modifier…".

**Spell** ([spells-xphb.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/spells/spells-xphb.json), 391 spells). The structured fields are `level`, `school`, `time` (including `unit:"bonus"` and a free-text `condition`), `range`, `components`, `duration` (concentration flag), `damageInflict`, `savingThrow`, `spellAttack`, `scalingLevelDice` (cantrips), `miscTags` and `areaTags`.

Damage amounts are not structured; they appear only as tags inside the prose. Searing Smite, for example, says "an extra `{@damage 1d6}` Fire damage" and scales by "`{@scaledamage 1d6|1-9|1d6}`".

**Species and backgrounds** live in `races.json` and `backgrounds.json`. The 2024 entries are `source: "XPHB"`.

#### 1.3 Tags: what is and isn't machine-readable

Inline tags are renderer markup (`{@tag text|source|display}`). I counted tag usage across all 2024 (`XPHB`) feature records of the six MVP classes: 325 records, about 18.7k words.

| Tag | Count |
|---|---|
| `variantrule` | 368 |
| `spell` | 85 |
| `condition` | 83 |
| `action` | 59 |
| `feat` | 34 |
| `item` | 34 |
| `filter` | 31 |
| `dice` | 27 |
| `damage` | 17 |
| `dc` | 7 |
| `itemMastery` | 5 |

Only 31 of the 325 records contain any `dice`, `damage`, `hit`, `d20` or `scale*` tag. (Method: a Python count over the downloaded JSON files, in a scratch script that was not kept.)

The tags are **hyperlinks and dice-roller hints, not semantics**. They mark that "1d6" is rollable. They do not say when it applies, to what, how often, or what it costs (action, bonus action, reaction, resource). Triggers ("once per turn", "when you hit with a Heavy weapon as part of the Attack action") and action-economy constraints, which the spec's solver depends on, exist only as English.

#### 1.4 Partial automation data: `foundry-*.json`

5e.tools ships Foundry-VTT-shaped automation overlays for its Plutonium importer, for example [`data/class/foundry.json`](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/class/foundry.json). It covers 163 distinct 2024 MVP-class features in some form. Examples:

- Champion Improved Critical becomes `flags.dnd5e.weaponCriticalThreshold DOWNGRADE 19`.
- Rage becomes `system.bonuses.mwak.damage ADD @scale.barbarian.rage-damage`, plus damage resistances.
- Sneak Attack becomes a damage activity with `@scale.rogue.sneak-attack` and a once-per-turn recovery.

Coverage is uneven. Reckless Attack, for instance, has `"changes": []`. All of it targets the Foundry actor data model, not a probability engine. It is useful as a cross-check, not as an engine input, and it carries the same licensing problem as the rest of the corpus.

#### 1.5 How 2014 and 2024 are distinguished

- **Source codes** ([books.json](https://github.com/5etools-mirror-3/5etools-src/blob/main/data/books.json)):
  - `PHB` = Player's Handbook (2014), published 2014-08-19.
  - `XPHB` = Player's Handbook (2024), 2024-09-17.
  - `XDMG` = Dungeon Master's Guide (2024), 2024-11-12.
  - `XMM` = Monster Manual (2025), 2025-02-18.
  - Later 2024-era books in the data include `FRHoF` (Forgotten Realms: Heroes of Faerûn, 2025-11-11), `EFA` (Eberron: Forge of the Artificer, 2025-12-09), `RHW` (2026-06-16), `AU` (Arcana Unleashed, 2026-09-15) and `WoW` (2026-11-17).
- **`edition`** is `"classic"` or `"one"`. The schema calls it "a context-sensitive behaviour hint" ([util-edition.json](https://github.com/TheGiddyLimit/5etools-utils/blob/main/schema/site/util-edition.json)). In practice, 2014 Fighter is `PHB/classic` and 2024 Fighter is `XPHB/one`.
- **SRD and basic-rules flags:** `srd` / `basicRules` mark 2014 content and `srd52` / `basicRules2024` mark 2024 content. Each is `boolean | string`, where a string means the SRD uses a different name ([class.json schema](https://github.com/TheGiddyLimit/5etools-utils/blob/main/schema/site/class/class.json)).
  - I spot-checked `srd52` against the SRD 5.2.1 PDF: the feats, MVP subclasses, smite and strike spells, masteries, species and backgrounds all matched (§2.3).
  - This makes `srd52 === true` a practical filter for *identifying* SRD content. It is still not a license to use 5e.tools' copy of it.
- **Cross-edition pollution hazard.** The Fighter file lists 10 subclasses with `classSource: "XPHB"` but no `edition`. Examples: Samurai (XGE), Echo Knight (EGW), Rune Knight (TCE), and the 2014 PHB Battle Master. These are renderer-generated "synthetic subclass copies" (`_meta.internalCopies: ["subclass"]`). The edition schema doc notes "an edition mismatch between class and subclass will prompt the renderer to add a note … feature levels may have to be adjusted". Any importer must reject these to honor the spec's "Cross-edition rule pollution … explicitly prohibited".

### 2. Legal status

#### 2.1 SRD releases

From the official SRD page ([dndbeyond.com/srd](https://www.dndbeyond.com/srd)):

- **SRD 5.2.1** was released 2025-05-01 under CC-BY-4.0, with errata. Its predecessor, SRD 5.2.0, came out 2025-04-22. The page says future versions (for example 5.2.2) will follow errata and will use CC-BY-4.0 exclusively.
  - PDF: <https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf>
  - Conversion guide (5.1 to 5.2.1): <https://media.dndbeyond.com/compendium-images/srd/guide/converting-to-srd-5.2.1.pdf>
- **SRD 5.1 (2014 rules)** is available under both CC-BY-4.0 and OGL 1.0a.
- **Commercial use.** The page says CC-BY permits commercial use if attribution is followed.
- **Required attribution** (from the SRD 5.2.1 PDF, p.1): "This work includes material from the System Reference Document 5.2.1 ("SRD 5.2.1") by Wizards of the Coast LLC, available at https://www.dndbeyond.com/srd. The SRD 5.2.1 is licensed under the Creative Commons Attribution 4.0 International License…". The PDF also says "Please do not include any other attribution to Wizards…", but allows "compatible with fifth edition" / "5E compatible".
- **CC-BY-4.0 terms** ([legal code](https://creativecommons.org/licenses/by/4.0/legalcode)):
  - attribution
  - indicate modifications
  - no additional restrictions on recipients' exercise of licensed rights (§2(a)(5)(B))
  - no share-alike, so our own code and encodings can stay proprietary.
- **Out of scope.** The SRD page says the SRD and CC license do not cover streaming, fan art or cosplay; the Fan Content Policy covers those.

#### 2.2 What SRD 5.2 adds, renames and leaves out

According to [dndbeyond.com/srd](https://www.dndbeyond.com/srd):

- **Added vs 5.1:** 3 backgrounds (Criminal, Sage, Soldier), 2 species (Goliath, Orc), 15 feats, 20 spells, 15 magic items and 17 monsters, plus a Rules Glossary.
- **Removed:** Half-Elf and Half-Orc.
- **Renamed:** *Deck of Many Things* became *Mysterious Deck*.
- **Excluded:** for example the Artificer, the Aasimar species, the Beholder, and names like Strahd and Tiamat.

#### 2.3 SRD 5.2.1 contents relevant to the MVP

I verified these against the SRD 5.2.1 PDF text: the table of contents (pp. 2–3) and the headings in the Classes, Character Origins, Feats (p.87–88), Equipment / Mastery Properties (p.90) and Spells chapters. The 5e.tools `srd52` flags agree on every item checked.

| Area | In SRD 5.2.1 | Not in SRD 5.2.1 (2024 PHB) |
|---|---|---|
| **Classes** | All 12, full base-class features: Fighter, Rogue, Barbarian, Ranger, Paladin, Monk, etc. This includes Weapon Mastery, Cunning Strike, Brutal Strike and the 2024 Divine Smite spell. | — |
| **Subclasses (MVP)** | **One each:** Champion (Fighter), Thief (Rogue), Path of the Berserker (Barbarian), Hunter (Ranger), Oath of Devotion (Paladin), Warrior of the Open Hand (Monk) | Fighter: Battle Master, Eldritch Knight, Psi Warrior. Rogue: Arcane Trickster, Assassin, Soulknife. Barbarian: Wild Heart, World Tree, Zealot. Ranger: Beast Master, Fey Wanderer, Gloom Stalker. Paladin: Glory, Ancients, Vengeance. Monk: Mercy, Shadow, Elements. |
| **Feats** | 17 total. Origin: Alert, Magic Initiate, Savage Attacker, Skilled. General: Ability Score Improvement, Grappler. Fighting Style: Archery, Defense, Great Weapon Fighting, Two-Weapon Fighting. Epic Boons: Combat Prowess, Dimensional Travel, Fate, Irresistible Offense, Spell Recall, Night Spirit, Truesight. | **Damage-relevant:** Great Weapon Master, Sharpshooter, Polearm Master, Crossbow Expert, Dual Wielder, Sentinel, Shield Master, Charger, Mage Slayer, Piercer, Slasher, Crusher, Tavern Brawler, Fey-Touched, Shadow-Touched, Elemental Adept, Spell Sniper, War Caster, Weapon Master, Martial Weapon Training, Heavy Armor Master, Defensive Duelist, Resilient, Tough, Lucky, Athlete, Speedy, Poisoner, Telekinetic. **Fighting styles:** Dueling, Blind Fighting, Interception, Protection, Thrown Weapon Fighting, Unarmed Fighting, Blessed Warrior, Druidic Warrior. **Epic boons:** Boon of Energy Resistance, Fortitude, Recovery, Skill, Speed. (60 non-SRD XPHB feat records in 5e.tools; 17 SRD.) |
| **Weapon masteries** | All 8: Cleave, Graze, Nick, Push, Sap, Slow, Topple, Vex | — |
| **Weapons** | All 40 PHB weapons (5e.tools: 40/40 XPHB weapons `srd52`) | — |
| **Species** | Dragonborn, Dwarf, Elf, Gnome, Goliath, Halfling, Human, Orc, Tiefling | Aasimar |
| **Backgrounds** | Acolyte, Criminal, Sage, Soldier | The other 12 (Artisan, Charlatan, Entertainer, Farmer, Guard, Guide, Hermit, Merchant, Noble, Sailor, Scribe, Wayfarer). These matter because a 2024 background fixes the origin feat and ability score options. |
| **Spells (MVP-relevant)** | Divine Smite, Searing Smite, Shining Smite, Divine Favor, Hunter's Mark, Ensnaring Strike, Bless, Haste, Magic Weapon, Find Steed, True Strike (5e.tools: 339/391 XPHB spells `srd52`) | Thunderous Smite, Wrathful Smite, Blinding Smite, Staggering Smite, Banishing Smite, Hail of Thorns, Lightning Arrow, Conjure Barrage, Swift Quiver, Steel Wind Strike, Elemental Weapon, Crusader's Mantle, Summon Beast |

Two consequences follow from the table:

- **Backgrounds.** With SRD-only backgrounds, the only origin feats reachable are Magic Initiate (Acolyte), Alert (Criminal), Magic Initiate (Sage) and Savage Attacker (Soldier). This is from the SRD background entries. *Verify the exact feat per background against SRD p.83 before relying on it.*
- **Optimization ceiling.** An SRD-only MVP **cannot represent the canonical 2024 martial optimization builds**: GWM, Polearm Master or Sharpshooter Fighters, Crossbow Expert hand-crossbow, Dual Wielder Nick builds, Battle Master, Gloom Stalker, Vengeance Paladin, and so on. This directly undercuts the spec's thesis that the solver "reveals stronger builds … than static guides".

**2014 (P1) parallel.** In 5e.tools, the SRD 5.1 flags (`srd: true`) on the 2014 PHB content show one MVP subclass each (Champion, Thief, Berserker, Hunter, Devotion, Way of the Open Hand) and exactly **one feat (Grappler)**. The 2014 SRD therefore lacks GWM, Sharpshooter, PAM and Crossbow Expert, which the spec's P1 sandbox explicitly names. *(Derived from 5e.tools flags; not re-verified against the SRD 5.1 PDF.)*

#### 2.4 WotC Fan Content Policy

Source: [company.wizards.com/en/legal/fancontentpolicy](https://company.wizards.com/en/legal/fancontentpolicy), last updated 2017-11-15.

- **Free:** "Free means FREE: You can't require payments, surveys, downloads, subscriptions, or email registration to access your Fan Content". "You can't sell or license your Fan Content to any third parties for any type of compensation".
- **Monetization:** sponsorships, ads and donations are allowed "so long as it doesn't interfere with the Community's access".
- **Mechanics:** "You cannot incorporate Wizards patents, game mechanics (unless your Fan Content is created under the D&D Open Game License)…". Logos and trademarks need written permission.
- **Revocable:** "We have the right to stop or restrict your use of Wizards' IP at any time—for any reason or no reason", and WotC decides "in our sole discretion".
- **Software:** the policy does not address apps or software specifically.

**Implication.** The Fan Content Policy is a permission, not a license, and it does not cover a gated or paid product. Its "game mechanics" carve-out makes it a poor foundation for a mechanics engine. Any freemium, account-gated or paid tier would fall outside it.

#### 2.5 Copyright scope: mechanics vs expression

- **US Copyright Office** ([Games registration guidance](https://www.copyright.gov/register/tx-games.html)): "The idea for a game is not protected by copyright. The name or title given to the game and the methods for playing it are not protected by copyright." However, "you may be able to register the text describing the rules of the game."
- **Engineering reading (needs counsel).** A numeric/logical encoding written in our own schema is closer to an unprotected "method of play" than to copied text. An example would be `{id:"2024:feat/great-weapon-master", trigger:"hit", requires:["weapon.heavy","action.attack","own-turn"], effect:{bonusDamage:"PB"}}`. Copying WotC prose (as the 5e.tools corpus does) is not.
- **Residual risks:**
  - feature and spell *names*: short phrases are generally not copyrightable, but trademark questions remain;
  - selection and arrangement of a full catalog;
  - non-US jurisdictions (*unverified*);
  - WotC's demonstrated willingness to send takedowns regardless of merit.

#### 2.6 Enforcement precedent against 5e.tools

- **2024 notice.** GitHub's public DMCA repo has a notice dated 2024-08-07 from counsel for Wizards of the Coast ([2024-08-07-wizards-of-the-coast.md](https://github.com/github/dmca/blob/master/2024/08/2024-08-07-wizards-of-the-coast.md)). It targets `5etools-mirror-1.github.io` and `5etools-mirror-2.github.io`, describes them as consisting "almost exclusively of content copied verbatim from Wizards' D&D publications", and quotes the contributor rule above as evidence. The notice also claims copyright in "details on characters, classes, races, feats, spells…".
- **Earlier notice.** A 2017 WotC notice against an earlier 5etools GitHub Pages host is reported in secondary mirrors of the DMCA repo (*not verified at the primary source*).
- **Current state.** The `5etools-mirror-3/5etools-src` repo is currently live. Its legal status is not established by that fact.

**Risk assessment for MinMax5e ingesting 5e.tools data:**

| Use | Risk |
|---|---|
| Bundle or host the 5e.tools JSON (or derived text) in a public app or repo | **High.** Same pattern as the takedown. |
| Ingest at build time, ship only derived numbers plus names | **Medium.** The derivation is still from an infringing copy, and names and catalog structure are exposed. |
| Local, user-initiated import of user-held files, never redistributed | **Lower**, but contributory-infringement theories exist if the product is designed around and marketed for a pirate source (*not verified; counsel*). |
| Hand-authored mechanics encodings, own wording, no WotC text, SRD text only where CC-BY | **Lowest practical risk** for non-SRD coverage. |

### 3. Alternatives

| Source | 2024 coverage | Structure | License | Freshness |
|---|---|---|---|---|
| **SRD 5.2.1 PDF** ([dndbeyond.com/srd](https://www.dndbeyond.com/srd)) | Canonical SRD 5.2.1 | PDF only; must be parsed or hand-transcribed | CC-BY-4.0 | Official; errata versions are planned |
| **5e-bits / dnd5eapi** ([5e-database](https://github.com/5e-bits/5e-database), [API `/api/2024`](https://www.dnd5eapi.co/api/2024)) | SRD 5.2: 12 subclasses (one per class), 17 feats, 339 spells, 232 features, weapon mastery properties, species, backgrounds | Clean typed JSON for equipment (dice, properties, mastery) and spells (`damage_at_slot_level`, `concentration`). Feature mechanics are a prose `description` string. Separate `src/2014` and `src/2024` trees. | Code MIT; content is SRD (CC-BY), so attribution is still needed | DB pushed 2026-10-02; API pushed 2026-11-23 |
| **Open5e** ([open5e-api](https://github.com/open5e/open5e-api), [`/v2/documents`](https://api.open5e.com/v2/documents/)) | Document `srd-2024` ("System Reference Document 5.2", CC-BY-4.0, gamesystem `5e-2024`): 17 feats; classes plus the 12 SRD subclasses; weapons with mastery as a typed property | v2 REST. Class features are `{key,name,desc,feature_type,gained_at}`, so mechanics are prose. Every record carries `document` and `gamesystem` provenance. Also hosts third-party OGL/CC 5e content (Kobold Press, A5E, Black Flag) | "Modified MIT", which explicitly excludes "SRD content provided by 3rd-parties"; per-document licenses are exposed in the API | Pushed 2026-11-24 |
| **Foundry VTT `dnd5e` system** ([repo](https://github.com/foundryvtt/dnd5e), [system.json](https://github.com/foundryvtt/dnd5e/blob/master/system.json)) | SRD 5.1 packs plus 2024 packs (`classes24`, `feats24`, `spells24`, `equipment24`, `origins24`), i.e. SRD 5.2 | Foundry documents with Activities and Active Effects: partial, Foundry-specific automation | Code MIT; content from SRD 5.1/5.2 under CC-BY-4.0 per README; assets vary | Release 6.1.1, 2026-11-16 |
| **Foundry premium "D&D Player's Handbook"** ([package](https://foundryvtt.com/packages/dnd-players-handbook)) | Full 2024 PHB (listing: 48 subclasses, 75 feats, 391 spells, per a [third-party mirror](https://foundryvtt-hub.com/package/dnd-players-handbook)) | Foundry documents | Paid, licensed, non-redistributable (*price and terms not verified at the primary source*) | — |
| **D&D Beyond** | Full official catalog | **No public API.** Staff have said there is no public-facing API and that site JSON is not meant as one ([forum thread](https://www.dndbeyond.com/forums/d-d-beyond-general/d-d-beyond-feedback/188822-d-d-beyond-api); *seen via search snippet only*). | [Terms & Conditions](https://www.dndbeyond.com/terms-conditions) §5.2 prohibits "automation software, bots … spiders … scripts … extraction tools" and "reverse engineering" (updated 2026-06-25) | — |
| **5e.tools** | Everything, including 2025–2026 books | Best metadata structure of any source; mechanics are prose with display tags | Code MIT; content is WotC copyright, DMCA precedent | Pushed 2026-11-23 |
| **BYO / homebrew import** (5e.tools homebrew format, [TheGiddyLimit/homebrew](https://github.com/TheGiddyLimit/homebrew); Foundry compendium import) | Whatever the user supplies | 5e.tools schema, Foundry documents | User's responsibility, if it stays local and is not redistributed | — |

**Takeaway:** every source is prose for feature mechanics. The SRD-derived APIs mainly save us PDF transcription of metadata (weapons, spells, class tables). The rule graph is our own work under any option.

## 4. Recommendation

1. **Baseline content: SRD 5.2.1 (2024) and SRD 5.1 (2014, P1).**
   - Ingest names, metadata and short descriptions from SRD 5.2.1. The practical route is to use 5e-bits or Open5e JSON as the transcription aid and keep the PDF as the authority.
   - Ship the exact CC-BY attribution statement in the UI and repo. Record `provenance: "SRD-5.2.1 (CC-BY-4.0)"` per record.
2. **The product asset: project-authored mechanics encodings.**
   - A separate rule-graph layer holds triggers, conditions, effects, action costs, per-turn limits and resource pools. It is keyed by stable IDs and contains *no WotC prose*.
   - Encode SRD content first (fully legitimate).
   - Treat non-SRD MVP essentials as an explicit, counsel-reviewed decision: GWM, Sharpshooter, PAM, Crossbow Expert, Dual Wielder, Battle Master, Gloom Stalker, Vengeance, Thunderous/Wrathful Smite and similar. Those records would be name plus numbers plus our own one-line summary, never book text.
3. **Optional local-only "bring your own content" import.**
   - The user points the app at their own files (5e.tools-format homebrew JSON, or a Foundry compendium export). Parsing happens client-side; nothing is uploaded or redistributed.
   - Imported content gets `provenance: "user-import"` and is invisible to any shared or published build.
   - Imported prose is *display only*. Executable logic still comes from the encoding layer or a user-authored encoding.
   - The app does not fetch from 5e.tools or D&D Beyond itself.
4. **2014 vs 2024 partition.**
   - Make `ruleset ∈ {2024, 2014}` the first segment of every content ID (for example `2024:class/fighter`, `2014:feat/great-weapon-master`).
   - Each ruleset is a separately loaded content pack.
   - Validation rejects any reference that crosses rulesets.
   - Map 5e.tools sources by an explicit allow-list: `XPHB`, `XDMG`, `XMM` and later `edition:"one"` books become 2024; `PHB`, `DMG`, `XGE`, `TCE`, etc. become 2014.
   - Drop records with a missing or mismatched `edition`, including synthetic subclass copies.
   - Same-named entities (Champion 2014 vs 2024) are *different entities*, not versions of one entity.
5. **Keep the spec honest.** Replace "Complete 5e.tools Corpus" in the spec's metadata with "SRD 5.2.1 baseline plus authored mechanics plus optional local import". Scope MVP marketing claims to what is encodable.

## Open questions

1. **Legal (blocking for any public release):** Can MinMax5e ship hand-authored mechanical encodings, with names, of non-SRD content (GWM, Battle Master, etc.) without a WotC license? Does the answer change between a free, ungated tool and a paid or freemium one? *Needs counsel. Not answerable from primary sources alone.*
2. Is the product free and ungated, so the Fan Content Policy is at least arguable for display text, or will it ever be commercial, so CC-BY SRD plus original work only? This decides how much non-SRD exposure is tolerable.
3. Is an SRD-only MVP still worth building as a technical proof (the engine and dashboard work; the build quality is unrepresentative), with non-SRD packs added after legal review?
4. Does a BYO-import feature that *documents* the 5e.tools format create contributory-infringement exposure? *Unverified; counsel.*
5. What is the license of the 5e.tools JSON *schema* (5etools-utils has no LICENSE file)? Can we reuse it for the import parser, or should we write our own reader?
6. Background origin-feat mapping in SRD 5.2.1 (Acolyte, Criminal, Sage, Soldier): confirm exact feats and ability options from SRD p.83.
7. How will we track SRD errata (5.2.2 and later) and record which SRD version each record was checked against?
8. The WotC terms that cover the 2024 Basic Rules on D&D Beyond (`basicRules2024` in 5e.tools) were not researched. Is anything there more permissive than the SRD? *Probably not; unverified.*

## Candidate ADRs

**ADR candidate 1: Do not use the 5e.tools corpus as a shipped or build-time data source.**
- *Decision:* MinMax5e will not bundle, host, build from or auto-fetch 5e.tools data. Content comes from SRD 5.1 / 5.2.1 under CC-BY-4.0 plus project-authored mechanics encodings. This reverses the spec's stated data source.
- *Rationale:* the corpus is predominantly verbatim non-SRD WotC text, it was the subject of a 2024 WotC DMCA notice, and its mechanics are prose anyway, so it saves little engineering work.
- *Rejected:* (a) ingest 5e.tools at build time and ship derived data; (b) runtime fetch from 5e.tools; (c) rely on the Fan Content Policy. It excludes "game mechanics", requires the product to be free, and is revocable at will.

**ADR candidate 2: Mechanics live in a hand-authored rule-graph layer, separate from descriptive content.**
- *Decision:* executable logic (triggers, conditions, effects, action costs, limits) is authored in our own schema and keyed by content ID. Descriptive text and metadata are a separate, licensable layer that the engine never parses.
- *Rationale:* no available source encodes mechanics (all are prose plus display tags). Separation also lets the legally sensitive layer (text) be swapped or omitted without touching the engine.
- *Rejected:* NLP or regex parsing of `entries` prose into rules (the spec's implied approach): fragile, unverifiable, and it ties the engine to a text source we cannot ship. Also rejected: adopting Foundry Active Effects as the rule model, which is Foundry-actor-specific and only partly covers the needed features.

**ADR candidate 3: Ruleset is part of every content ID, with no cross-ruleset references.**
- *Decision:* content IDs are `<ruleset>:<kind>/<slug>` with `ruleset ∈ {2024, 2014}`, loaded as separate packs. The loader rejects cross-ruleset references and 5e.tools-style synthetic cross-edition copies.
- *Rationale:* this enforces the spec's "hard boundaries" against cross-edition pollution at the data layer rather than in solver logic. Same-named entities (2014 vs 2024 Champion, GWM) differ mechanically.
- *Rejected:* a single entity with an `edition` attribute or versions (5e.tools' model, which allows synthetic mixing); keying by book source code (sources do not map 1:1 to rulesets as new books ship).

**ADR candidate 4: User-supplied content stays local and is never redistributed.**
- *Decision:* any import of user-held rules data is parsed and stored only on the user's device. It is tagged `user-import` and excluded from shared or published builds and server features.
- *Rationale:* this is the only way to support full-catalog play without MinMax5e itself distributing non-licensed content.
- *Rejected:* a server-side upload or shared library of user imports (makes us a distributor); no import at all (forces SRD-only forever).

## Glossary candidates

| Term | Definition | Synonyms to avoid |
|---|---|---|
| **Ruleset** | One of the two mutually exclusive rule environments, `2024` (P0) or `2014` (P1), that scopes every piece of content and every evaluation. | edition, version, "5.5e", "One D&D" |
| **SRD** | Wizards' System Reference Document. Here, specifically SRD 5.2.1 (2024 rules) or SRD 5.1 (2014 rules), released under CC-BY-4.0. | basic rules, free rules |
| **Content record** | A licensable, descriptive entry (name, metadata, short text) for a class, subclass, feature, feat, spell, item, species or background, carrying a provenance tag. | entity, data entry, compendium item |
| **Mechanics encoding** | The project-authored, executable representation of what a content record does (triggers, conditions, effects, costs, limits), with no copied rulebook text. | rule, parsed feature, automation, effect script |
| **Rule graph** | The full set of mechanics encodings for a ruleset, which the solver evaluates. | rules engine data, knowledge base |
| **Provenance** | The tag on each content record stating where it came from and under what terms (`SRD-5.2.1`, `SRD-5.1`, `authored`, `user-import`). | source (ambiguous with book source codes), origin (clashes with 2024 "Origin" feats) |
| **Content pack** | A loadable bundle of content records and encodings for exactly one ruleset and one provenance. | module, expansion, book |
| **User import** | Content the user supplies from their own files, parsed locally, never redistributed. | homebrew (narrower), BYOD, upload |
| **Source code (book)** | A 5e.tools-style short ID for a publication (`XPHB`, `PHB`, `XDMG`). Used only for import mapping, never as a ruleset key. | source, book |
