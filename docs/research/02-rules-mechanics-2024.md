# 02 — 2024 Rules Mechanics the MVP Must Model (and a fact-check of the spec)

*Researched 2026-11-26. Scope: the 2024 rules and the MVP roster (Fighter, Rogue, Barbarian, Ranger, Paladin, Monk), single-target sustained and nova DPR. It also covers the 2014 contrasts that the spec's sandbox relies on.*

## Question

Which 2024 (and contrasting 2014) D&D 5e rules must the MVP engine model exactly, what does the official text say, and which rules claims in [`docs/spec.md`](../spec.md) are correct, imprecise or wrong?

## Sources and citation keys

| Key | Source | Notes |
|---|---|---|
| **[SRD p.N]** | *System Reference Document 5.2.1*, CC-BY-4.0, WotC. https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.1.pdf | I downloaded it and extracted the text. Page numbers are the printed SRD pages. SRD 5.2 (https://media.dndbeyond.com/compendium-images/srd/5.2/SRD_CC_v5.2.pdf) was also downloaded. 5.2.1 already includes the PHB errata ([ERR] says the errata was "already implemented ... in SRD 5.2"). |
| **[5ET:file]** | 5e.tools data on GitHub, `5etools-mirror-3/5etools-src` @ `main`, e.g. https://github.com/5etools-mirror-3/5etools-src/blob/main/data/feats.json | Used **only as a cross-check of 2024 PHB (source code `XPHB`) text that the SRD lacks**, and for 2014 PHB/XGE/TCE text. It is a community transcription, not an official source. Files used: `items-base.json`, `feats.json`, `actions.json`, `variantrules.json`, `class/class-{fighter,rogue,barbarian,ranger,paladin,monk}.json`, `spells/spells-{xphb,phb,tce}.json`, `backgrounds.json`, `races.json`, `book/book-phb.json`, `generated/gendata-spell-source-lookup.json`. Wherever SRD and 5e.tools overlap, the wording I checked matched. |
| **[SAC:id]** | Sage Advice Compendium (2024 rules), D&D Beyond. https://www.dndbeyond.com/sources/dnd/sae/sage-advice-compendium | Ids such as `SAC-Rogue1` are the anchors on that page. SAC is a FAQ, not errata, and does not override the DM. The fetch tool returned paraphrases, so wording is near-verbatim, not exact. |
| **[ERR]** | Player's Handbook (2024) errata. https://www.dndbeyond.com/sources/dnd/sae/players-handbook | Posted 2025-04-30. The index page shows "Last Updated: Oct 7, 2026", but I could not see any items newer than the April 2025 set (**unverified**). |
| **[DDB-WM]** | "Your Guide to Weapon Mastery in the 2024 Player's Handbook", D&D Beyond, 2024-06-18 (updated 2024-08-12). https://www.dndbeyond.com/posts/1742-your-guide-to-weapon-mastery-in-the-2024-players | Official editorial content, not a rules text. |
| **[COMPAT]** | ComicBook.com interviews with Jeremy Crawford on backward compatibility: https://comicbook.com/gaming/news/dungeons-dragons-backwards-compatibility-2024-players-handbook and https://comicbook.com/gaming/news/dungeons-dragons-2024-core-rulebooks-backwards-compatibility | A secondary source reporting official statements. |

I did not fetch the D&D Beyond Free Rules (2024) pages (https://www.dndbeyond.com/sources/dnd/free-rules). SRD 5.2.1 carries the same rules text for everything cited here.

---

## TL;DR

1. **Weapon Mastery** is per weapon *kind* and needs a class feature or the Weapon Master feat. Fighter gets 3/4/5/6 kinds at levels 1/4/10/16, swapping **one** per long rest. Barbarian gets 2/3/4 at 1/4/10 (melee only), swapping **one**. Rogue, Ranger and Paladin get a fixed **2** and may change **all** of them per long rest. **The 2024 Monk has no Weapon Mastery feature.** [5ET:class-*.json; SRD p.28, 48, 58, 62]
2. **Nick** moves the Light property's extra attack into the Attack action. The Light property itself still needs (a) the Attack action, (b) an attack with a Light weapon and (c) a *different* Light weapon. The extra attack adds no ability modifier unless the Two-Weapon Fighting style feat applies. Nick is once per turn. Together with **Dual Wielder** it yields Attack-action attacks + a Nick attack + a Bonus Action attack. [SRD p.89–90; 5ET:feats.json]
3. **Weapon swapping:** each attack made as part of the Attack action lets you equip *or* unequip one weapon, before or after that attack [SRD p.177]. On top of that you get **one free object interaction per turn** [SRD p.13]. Dual Wielder's Quick Draw doubles each draw/stow. Thrown weapons can be drawn as part of their own attack. The canonical "Vex then Nick" line (Shortsword, Vex + Light, with a Scimitar or Dagger, Nick + Light) needs **no swap at all**.
4. **Spell economy:** "On a turn, you can expend only one spell slot to cast a spell" [SRD p.105]. This applies on *any* creature's turn. Casts that need no slot (Paladin's Smite, the Ranger's Favored Enemy Hunter's Mark, Fey-Touched, Magic Initiate) are exempt. **Divine Smite is now a level 1 spell** cast as a **Bonus Action immediately after hitting with a Melee weapon or Unarmed Strike** [SRD p.125]. It therefore cannot be cast on reaction attacks, while Raging, or twice in a turn.
5. **Action Surge:** "one additional action, except the Magic action" [SRD p.48]. The spec is correct on this point.
6. **GWM 2024:** extra damage equal to Proficiency Bonus on a hit with a **Heavy** weapon **as part of the Attack action on your turn**, with no attack penalty. That condition excludes Hew, Pole Strike, Opportunity Attacks and Light extra attacks. It *does* include Heavy ranged weapons (Longbow, Heavy Crossbow). **Sharpshooter 2024 has no damage bonus at all.** [5ET:feats.json]
7. **SRD vs PHB-only:** the SRD contains only the Origin feats Alert, Magic Initiate, Savage Attacker and Skilled; the General feats ASI and Grappler; the Fighting Styles Archery, Defense, Great Weapon Fighting and Two-Weapon Fighting; 7 Epic Boons; one subclass per class; and 4 backgrounds. **GWM, Sharpshooter, Dual Wielder, Charger, Mage Slayer, Polearm Master, Crossbow Expert, Fey-Touched and Dueling are PHB-only.** [SRD p.87–88]
8. **Spec errors to fix:** the loadout example "Vex Dagger" is **wrong** (Dagger = Nick). **Spirit Shroud** is not a 2024 PHB or SRD spell; it is TCE, reachable by Paladins only through TCE's optional expanded list. **Hex** is Warlock-only, so for the MVP roster it is reachable only via Fey-Touched or a Warlock multiclass. **Booming Blade** is SCAG/TCE. "Free-action Divine Smite on crits" is not unique to 2014: 2024 smites are also chosen after the hit and still double on a crit. The full table is below.
9. The engine needs a **rulings register**. At least ten mechanics that matter for DPR are textually ambiguous (Nick weapon placement, Nick + Action Surge, whether Hunter's Mark/Hex dice double on a crit, Cleave/Nick inside the Attack action for GWM and equip purposes, and how multiclass Weapon Mastery features combine, among others).

---

## 1. Weapon Mastery

### 1.1 Exact text (SRD 5.2.1 p.90 "Mastery Properties"; identical in 5ET:items-base.json `itemMastery`)

> "Each weapon has a mastery property, which is usable only by a character who has a feature, such as Weapon Mastery, that unlocks the property for the character." [SRD p.90]

| Mastery | Text (verbatim, SRD p.90) | Engine notes |
|---|---|---|
| **Cleave** | "If you hit a creature with a melee attack roll using this weapon, you can make a melee attack roll with the weapon against a second creature within 5 feet of the first that is also within your reach. On a hit, the second creature takes the weapon's damage, but don't add your ability modifier to that damage unless that modifier is negative. You can make this extra attack only once per turn." | Multi-target only; it adds nothing to single-target DPR. It is unclear whether the extra attack is "part of the Attack action" (see E13). |
| **Graze** | "If your attack roll with this weapon misses a creature, you can deal damage to that creature equal to the ability modifier you used to make the attack roll. This damage is the same type dealt by the weapon, and the damage can be increased only by increasing the ability modifier." | A flat floor of the ability modifier on a miss. It cannot be increased by a +X weapon, Rage, GWM, Dueling and so on. Riders that need a hit cannot apply. Resistance still applies. |
| **Nick** | "When you make the extra attack of the Light property, you can make it as part of the Attack action instead of as a Bonus Action. You can make this extra attack only once per turn." | See §1.4. |
| **Push** | "If you hit a creature with this weapon, you can push the creature up to 10 feet straight away from yourself if it is Large or smaller." | Positional only. |
| **Sap** | "If you hit a creature with this weapon, that creature has Disadvantage on its next attack roll before the start of your next turn." | Defensive; no DPR effect. |
| **Slow** | "If you hit a creature with this weapon and deal damage to it, you can reduce its Speed by 10 feet until the start of your next turn. If the creature is hit more than once by weapons that have this property, the Speed reduction doesn't exceed 10 feet." | Positional only. |
| **Topple** | "If you hit a creature with this weapon, you can force the creature to make a Constitution saving throw (DC 8 plus the ability modifier used to make the attack roll and your Proficiency Bonus). On a failed save, the creature has the Prone condition." | **This mastery generates advantage.** Prone means "An attack roll against you has Advantage if the attacker is within 5 feet of you. Otherwise, that attack roll has Disadvantage." [SRD p.~186, Prone]. It needs the target's CON save profile as an input. |
| **Vex** | "If you hit a creature with this weapon and deal damage to the creature, you have Advantage on your next attack roll against that creature before the end of your next turn." | Advantage on the *next* attack roll against that creature only. That roll consumes it whether it hits or misses. Every Vex hit grants it again. |

### 1.2 Weapons by mastery (SRD p.91 weapon table; cross-checked with 5ET:items-base.json)

| Mastery | Weapons (category, damage, relevant properties) |
|---|---|
| Cleave | Greataxe (M, 1d12, Heavy, Two-Handed), Halberd (M, 1d10, Heavy, Reach, Two-Handed) |
| Graze | Glaive (M, 1d10, Heavy, Reach, Two-Handed), Greatsword (M, 2d6, Heavy, Two-Handed) |
| Nick | Dagger (S, 1d4, Finesse, Light, Thrown), Light Hammer (S, 1d4, Light, Thrown), Sickle (S, 1d4, Light), Scimitar (M, 1d6, Finesse, Light) |
| Push | Greatclub (S, 1d8, Two-Handed), Pike (M, 1d10, Heavy, Reach, Two-Handed), Warhammer (M, 1d8, Versatile), Heavy Crossbow (M, 1d10, Ammunition, Heavy, Loading, Two-Handed) |
| Sap | Mace (S, 1d6), Spear (S, 1d6, Thrown, Versatile), Flail (M, 1d8), Longsword (M, 1d8, Versatile), Morningstar (M, 1d8), War Pick (M, 1d8, Versatile) |
| Slow | Club (S, 1d4, Light), Javelin (S, 1d6, Thrown), Light Crossbow (S, 1d8, Loading, Two-Handed), Sling (S, 1d4), Whip (M, 1d4, Finesse, Reach), Longbow (M, 1d8, Heavy, Two-Handed), Musket (M, 1d12, Loading, Two-Handed) |
| Topple | Quarterstaff (S, 1d6, Versatile), Battleaxe (M, 1d8, Versatile), Lance (M, 1d10, Heavy, Reach, Two-Handed unless mounted), Maul (M, 2d6, Heavy, Two-Handed), Trident (M, 1d8, Thrown, Versatile) |
| Vex | Dart (S, 1d4, Finesse, Thrown), Handaxe (S, 1d6, Light, Thrown), Shortbow (S, 1d6, Two-Handed), Rapier (M, 1d8, Finesse), Shortsword (M, 1d6, Finesse, Light), Blowgun (M, 1, Loading), Hand Crossbow (M, 1d6, Light, Loading), Pistol (M, 1d10, Loading) |

The firearms (Pistol, Musket) appear in the SRD weapon table [SRD p.91].

### 1.3 Who gets mastery, how many, and long-rest swaps

| Class | Feature text (key clause) | Kinds by level | Swap on long rest | Source |
|---|---|---|---|---|
| Fighter | "three kinds of Simple or Martial weapons" | 3 (L1), 4 (L4), 5 (L10), 6 (L16) | "change **one** of those weapon choices" | SRD p.48; 5ET:class-fighter.json (table) |
| Barbarian | "two kinds of Simple or Martial **Melee** weapons" | 2 (L1), 3 (L4), 4 (L10) | change **one** | SRD p.28; 5ET:class-barbarian.json |
| Rogue | "two kinds of weapons of your choice with which you have proficiency" (proficiency = Simple, plus Martial weapons with Finesse or Light) | 2 | "you can change the kinds of weapons you chose" (**all**) | SRD p.62; 5ET:class-rogue.json; DDB-WM |
| Ranger | two kinds you are proficient with | 2 | change the kinds (**all**) | SRD p.58 |
| Paladin | two kinds you are proficient with | 2 | change the kinds (**all**) | SRD p.~54 |
| Monk | **none** | 0 | — | 5ET:class-monk.json (feature list L1–20) |
| Any (level 4+) | Weapon Master feat (PHB-only): mastery of one additional Simple or Martial weapon kind you are proficient with; +1 Str or Dex | +1 | change on long rest | 5ET:feats.json |

[DDB-WM] simplifies all classes to "swap out one of your weapon choices". For Rogue, Ranger and Paladin, the class text above is more permissive.

**Fighter L9 Tactical Master:** "When you attack with a weapon whose mastery property you can use, you can replace that property with the Push, Sap, or Slow property for that attack." [SRD p.48]
**Fighter L13 Studied Attacks:** "If you make an attack roll against a creature and miss, you have Advantage on your next attack roll against that creature before the end of your next turn." [SRD p.48] This gives Fighters a Vex-like effect on a miss.

### 1.4 Light property, Two-Weapon Fighting, Nick and Dual Wielder

- **Light** [SRD p.89]: "When you take the Attack action on your turn and attack with a Light weapon, you can make one extra attack as a Bonus Action later on the same turn. That extra attack must be made with a different Light weapon, and you don't add your ability modifier to the extra attack's damage unless that modifier is negative."
- **Two-Weapon Fighting (Fighting Style feat)** [SRD p.88]: "When you make an extra attack as a result of using a weapon that has the Light property, you can add your ability modifier to the damage of that attack if you aren't already adding it to the damage."
- **Nick**: see §1.1. It does **not** create a new attack. It only re-homes the Light extra attack from the Bonus Action into the Attack action. [DDB-WM]: "this doesn't mean you can make a third attack as a Bonus Action ... this frees up your Bonus Action to use class/species abilities, such as the Rogue's Cunning Action."
- **Dual Wielder (General feat, level 4+, Str or Dex 13, +1 Str or Dex; PHB-only)** [5ET:feats.json]:
  - *Enhanced Dual Wielding* (paraphrased): after attacking with a Light weapon during the Attack action on your turn, you may make one extra Bonus Action attack later that turn. It must use a *different* weapon, which has to be a Melee weapon without the Two-Handed property. The ability modifier is not added to its damage unless it is negative.
  - *Quick Draw* (paraphrased): whenever you could draw or stow one weapon, you may instead draw or stow two, provided neither is Two-Handed.
- **Sage Advice:** "The only requirement for the Light property's extra attack is that it's made with a different Light weapon". You may attack with one Light weapon, then draw a second and make the extra attack [SAC-Combat27]. Dual Wielder does not help Heavy weapons, because all PHB Heavy weapons are Two-Handed [SAC-Feat-DualWielder1].
- **Resulting maximum for a dual-wielder at Fighter 5** (Shortsword in one hand, Scimitar in the other, Dual Wielder): Attack 1 (Shortsword) + Attack 2 (Shortsword) + Nick extra attack (Scimitar, inside the Attack action) + Dual Wielder Bonus Action attack ("different weapon" than the triggering Light weapon, so the Scimitar is fine). That is **four attacks**. The Nick and Dual Wielder attacks add no ability modifier to damage unless the TWF style applies (whether TWF covers the Dual Wielder attack is ambiguous; see E6).
- **Crossbow Expert 2024** [5ET:feats.json; SAC-Feat-CrossbowExpert1]: it ignores Loading and removes the in-melee Disadvantage on crossbows. It also adds the ability modifier to a Light extra attack made with a Light crossbow. **There is no stand-alone Bonus Action hand-crossbow attack**: the Light extra attack still needs a *different* weapon, so you need two Hand Crossbows, or a Hand Crossbow plus a Light melee weapon.

---

## 2. Equipping, unequipping and swap chains

### 2.1 Rules text

- **Attack action → "Equipping and Unequipping Weapons"** [SRD p.177; 5ET:actions.json "Attack"]: "You can either equip or unequip one weapon when you make an attack as part of this action. You do so either before or after the attack. If you equip a weapon before an attack, you don't need to use it for that attack. Equipping a weapon includes drawing it from a sheath or picking it up. Unequipping a weapon includes sheathing, stowing, or dropping it."
- **Moving between attacks** [same]: movement can be split between attacks of the Attack action.
- **Free object interaction** [SRD p.13, "Interacting with Things"]: "You can interact with one object or feature of the environment for free, during either your move or action ... If you want to interact with a second object, you need to take the Utilize action."
- **Utilize** [5ET:actions.json, paraphrased]: object interactions normally happen alongside another activity. Its example is drawing a sword during the Attack action. The Utilize action is needed only for an interaction beyond that.
- **Thrown** [SRD p.89]: "...you can draw that weapon as part of the attack."
- **Ammunition** [SRD p.89]: "Drawing the ammunition is part of the attack (you need a free hand to load a one-handed weapon)." **Loading**: one piece of ammunition per action, Bonus Action or Reaction used to fire, unless Crossbow Expert applies.
- **Extra Attack** [SAC-CF-Gen1]: "Extra Attack imposes no limitation on what you use for the attacks." You can mix weapons and Unarmed Strikes, but the equip/unequip rules still apply.
- **Shields**: errata changed the Armor table heading to "Shield (Utilize Action to Don or Doff)" [ERR]. A shield cannot be swapped through the weapon-equip allowance.

### 2.2 The resource model this implies

For each turn, track:
1. **Hand state**: two hands. Each holds nothing, a one-handed weapon, half of a Two-Handed or Versatile weapon, or a shield.
2. **Per-attack equip token**: every attack made *as part of the Attack action* gives exactly one equip-or-unequip, used before or after that attack. Unused tokens are lost.
3. **One free object interaction** per turn, used during movement or during the action. It is not tied to a specific attack.
4. **Dual Wielder Quick Draw**: each draw or stow event can move two weapons that lack the Two-Handed property.
5. **Thrown draw**: drawing a Thrown weapon to throw it is free, as part of that attack.
6. **Dropped items persist.** A dropped weapon is on the floor until someone picks it up, which counts as an equip. Sustained rotations must therefore not assume a dropped weapon comes back.

### 2.3 Which swap sequences are legal (worked examples)

| # | Sequence | Legal? | Why |
|---|---|---|---|
| S1 | Hold Shortsword (Vex, Light) + Scimitar (Nick, Light). Attack 1 and Attack 2 with the Shortsword, then the Nick attack with the Scimitar. | **Yes, no swaps needed.** | The spec's "Vex then Nick" line. A Vex hit on Attack 2 gives the Nick attack advantage. |
| S2 | Hold only a Shortsword. Attack 1 Shortsword; equip a Dagger *after* Attack 1; Nick attack with the Dagger. | **Yes** | The equip token from Attack 1 draws the Dagger [SAC-Combat27]. |
| S3 | Two-handed Greatsword (Graze) for Attack 1, then Maul (Topple) for Attack 2. | **Yes, but the Greatsword is dropped or stowed.** | Attack 1's token: unequip (drop) the Greatsword after the attack. Attack 2's token: equip the Maul before the attack. Better order: Maul (Topple) first, then Greatsword with advantage against the Prone target. |
| S4 | As S3, and also draw the Greatsword back after Attack 2. | **Only with the free object interaction**, and only if it was stowed or is in reach. | Attack 2's token was spent on the Maul. Picking the Greatsword up is an equip, which needs another token or the free interaction. |
| S5 | Fighter 11 (three attacks) cycling three two-handed weapons, Topple → Graze → Cleave. | **Yes** if each previous weapon is dropped and each new one drawn: A1 Maul, drop after (token 1); draw Greatsword before A2 (token 2); after A2 nothing is left to drop it with unless the free interaction does it; draw Greataxe before A3 (token 3) **needs a free hand**. | The hand budget is the real constraint. The engine must solve this as a small planning problem rather than a fixed pattern. |
| S6 | Rapier (Vex, not Light) Attack 1, then a Light weapon for the Nick attack. | **No Nick attack** unless one of the Attack-action attacks was made **with a Light weapon** *and* the extra attack uses a *different* Light weapon. A1 Rapier, A2 Dagger, extra attack with a drawn Scimitar or Sickle works. | Light trigger condition [SRD p.89]. |
| S7 | Light extra attack as a Bonus Action (no Nick), with a weapon drawn just before it. | **Only via the free object interaction.** | The Bonus Action attack is not "part of the Attack action", so it gets no equip token. |
| S8 | Hand Crossbow + Hand Crossbow (Crossbow Expert): Attack, Attack, Light extra attack with the other crossbow. | **Yes** | Crossbow Expert ignores Loading and lets you load without a free hand [5ET:feats.json; SAC-Feat-CrossbowExpert1]. |
| S9 | Throwing a stack of Handaxes (Vex, Light, Thrown), one per attack, plus a Light extra attack with another Handaxe. | **Yes** | Thrown draw is part of each attack. The extra attack is "a different Light weapon", which a second Handaxe satisfies. |
| S10 | Nick attack *also* granting an equip token. | **Ambiguous (E7)** | It is "made as part of the Attack action", so it arguably counts as "an attack as part of this action". |

---

## 3. Spells, slots, concentration and action economy

### 3.1 One spell slot per turn

> "**One Spell with a Spell Slot per Turn.** On a turn, you can expend only one spell slot to cast a spell. This rule means you can't, for example, cast a spell with a spell slot using the Magic action and another one using a Bonus Action on the same turn." [SRD p.105]

- The limit applies **per turn, on any creature's turn**, so it covers Reaction spells too. A Reaction spell with a slot on your own turn is blocked if you already used a slot that turn [SAC-SpellCastingTime2].
- It limits **slot expenditure, not spell count**. A slotted Bonus Action spell plus an action cantrip is legal [SAC-SpellCastingTime1]. Casts that need no slot are exempt: Paladin's Smite (Divine Smite once per long rest without a slot [SRD p.~54]), Favored Enemy (Hunter's Mark without a slot, 2–6 times per long rest [SRD p.58]), Magic Initiate, Fey-Touched, and species spells.
- Spell attacks are not part of the Attack action; casting a spell on your turn takes the Magic action [SAC]. Extra Attack does not let you cast extra attack spells, and True Strike does not combine with Extra Attack [SAC].
- You can't cast a spell as an Opportunity Attack (except through War Caster) [SAC-SpellAttack5].

### 3.2 Divine Smite and the smite spells (2024)

**Divine Smite** [SRD p.125]: "Level 1 Evocation (Paladin). Casting Time: Bonus Action, which you take immediately after hitting a target with a Melee weapon or an Unarmed Strike. Range: Self. Component: V. Duration: Instantaneous. The target takes an extra 2d8 Radiant damage from the attack. The damage increases by 1d8 if the target is a Fiend or an Undead. Using a Higher-Level Spell Slot. The damage increases by 1d8 for each spell slot level above 1."
- There is **no cap** (the 2014 version capped at 5d8 / 6d8), so a level 5 slot gives 6d8.
- **Paladin's Smite (L2)**: "You always have the Divine Smite spell prepared. In addition, you can cast it without expending a spell slot, but you must finish a Long Rest before you can cast it in this way again." [SRD p.~54]
- **GWF does not apply to smite dice** [SAC-Feat-GreatWeaponFighting1]. **Savage Attacker does not reroll smite or Sneak Attack dice** [SAC-Feat-SavageAttacker1].

**Smite spells, 2024** [5ET:spells-xphb.json; SRD p.160 for Searing and Shining]. All are cast as a Bonus Action "immediately after hitting ... with a Melee weapon or an Unarmed Strike", except Ensnaring Strike ("a weapon") and Hail of Thorns ("a Ranged weapon").

| Spell | Lvl | Extra damage on the hit | Concentration? | In SRD 5.2? |
|---|---|---|---|---|
| Searing Smite | 1 | 1d6 Fire (+1d6 per slot level), plus 1d6 at the start of each of the target's turns until it saves | **No** (1 min) | Yes |
| Thunderous Smite | 1 | 2d6 Thunder (+2d6 per slot level), STR save or pushed 10 ft and Prone | No (instant) | No (PHB) |
| Wrathful Smite | 1 | 1d6 Necrotic (+1d6 per slot level), WIS save or Frightened | No (1 min) | No |
| Shining Smite | 2 | 2d6 Radiant (+2d6 per slot level), and **attacks against the target have Advantage** | **Yes** | Yes |
| Blinding Smite | 3 | 3d8 Radiant (+3d8 per slot level), target Blinded | No (1 min) | No |
| Staggering Smite | 4 | 4d6 Psychic (+4d6 per slot level), WIS save or Stunned | No (instant) | No |
| Banishing Smite | 5 | 5d10 Force | Yes | No |
| Divine Favor | 1 | +1d4 Radiant on weapon hits; Bonus Action cast, no trigger | **No** (1 min) | Yes [SRD p.~125] |
| Ensnaring Strike (Ranger) | 1 | Restrains, 1d6 per turn | Yes | Yes |
| Hail of Thorns (Ranger) | 1 | 1d10 area damage after a ranged hit | No | No |

The concentration flags come from 5ET duration data and should be spot-checked against the PHB. Because Divine Smite is a spell with a Bonus Action casting time:
- it can't be cast on another creature's turn, because a Bonus Action needs your own turn ([SRD p.~176] Bonus Action: "a special action that you can take on the same turn that you take an action"). **So no smites on Opportunity Attacks or other reaction attacks.**
- it is **one per turn** (one Bonus Action), and it competes with the Light extra attack, Hunter's Mark, Hex, Rage, Cunning Action, Flurry of Blows and so on.
- it **can't be cast while Raging** (Rage: "You can't maintain Concentration, and you can't cast spells" [SRD p.28]).
- it is still **chosen after the hit is known**, so smiting only on crits remains a valid nova policy in 2024.

### 3.3 Hunter's Mark, Hex, Spirit Shroud

- **Hunter's Mark** [SRD p.141]: Level 1 Divination (**Ranger**). Bonus Action; Concentration, up to 1 hour. "You deal an extra 1d6 **Force** damage to the target whenever you hit it with an **attack roll**", which includes spell attacks (2014 said "weapon attack"). "If the target drops to 0 Hit Points before this spell ends, you can take a Bonus Action to move the mark to a new creature." The 2024 text drops the 2014 "on a subsequent turn" limit [5ET:spells-phb.json vs SRD].
- **Ranger features built on Hunter's Mark** [SRD p.58–59; 5ET:class-ranger.json]: L1 *Favored Enemy* (always prepared; free casts 2/2/3/4/5/6 at levels 1/5/9/13/17); L13 *Relentless Hunter* ("Taking damage can't break your Concentration on Hunter's Mark"); L17 *Precise Hunter* ("Advantage on attack rolls against the creature currently marked"); L20 *Foe Slayer* (the die becomes a d10); Hunter subclass L11 *Superior Hunter's Prey* (spreads the Hunter's Mark damage once per turn) [SRD p.~59]. Hunter L3 *Colossus Slayer* adds 1d8 once per turn if the target is missing any Hit Points [SRD p.59].
- **Hex** [SRD p.140]: Level 1 Enchantment (**Warlock only** [5ET:gendata-spell-source-lookup.json]). 1d6 Necrotic per hit with an attack roll; Concentration up to 1 hour; moving the curse needs a Bonus Action "on a later turn". **None of the MVP classes has Hex on its list.** Fey-Touched (General feat, PHB-only) lets you choose one level 1 Divination or Enchantment spell, so Hunter's Mark *or* Hex, with one free cast per long rest plus slots [5ET:feats.json]. Magic Initiate (Cleric, Druid or Wizard lists) gives **neither**.
- **Spirit Shroud** [5ET:spells-tce.json]: Level 3 Necromancy from **Tasha's Cauldron of Everything (2020)**. It is **not in the 2024 PHB or SRD 5.2**. It is on the Paladin list only through TCE's optional class-variant spell list [5ET:gendata-spell-source-lookup.json `tce.spirit shroud.classVariant`], and needs Paladin 9 for 3rd-level slots. Text: Bonus Action, Concentration 1 min; +1d8 radiant, necrotic or cold on each hit against a creature within 10 ft of you (+1d8 per two slot levels above 3rd); targets can't regain HP.
- **Backward compatibility** [COMPAT]: Crawford's stated position is that older content not reprinted in the 2024 PHB remains usable, and reprinted content is replaced by its 2024 version. Spirit Shroud and Booming Blade are therefore "legal at a 2024 table" but **not 2024-core**.

### 3.4 Concentration [SRD p.179]

- "You lose Concentration on an effect the moment you start casting a spell that requires Concentration or activate another effect that requires Concentration."
- The damage save DC is "10 or half the damage taken (round down), whichever number is higher, up to a maximum DC of 30". Temporary HP do not reduce the DC [SAC].
- Concentration also ends when you become Incapacitated or die.
- Hunter's Mark, Hex, Spirit Shroud, Shining Smite, Banishing Smite, Ensnaring Strike, Elemental Weapon and Haste are mutually exclusive.
- **Rage ends the ability to concentrate**, so a Barbarian/Ranger multiclass drops Hunter's Mark on entering Rage.
- **Mage Slayer 2024**: damaged concentrators save with Disadvantage. There is **no reaction attack** any more [5ET:feats.json].

### 3.5 Action Surge, Magic action, Extra Attack

- **Action Surge** [SRD p.48]: "On your turn, you can take one additional action, except the Magic action. Once you use this feature, you can't do so again until you finish a Short or Long Rest. Starting at level 17, you can use it twice before a rest but only once on a turn." You can use both actions for the same action type (e.g., Dash twice) [SAC-Combat7].
- **Magic action** [5ET:actions.json, PHB p.371]: "cast a spell that has a casting time of an action or use a feature or magic item that requires a Magic action to be activated." Action Surge therefore also blocks True Strike, action-activated magic items and Ranger *Tireless* temp HP. It does **not** block Bonus Action or Reaction spells.
- **Extra Attack** [SRD p.48 and others]: attack twice when you take the Attack action. Fighter L11 makes it three attacks and L20 four. **Multiclass**: "If you gain the Extra Attack feature from more than one class, the features don't stack" [SRD p.25].
- **Second Attack action via Action Surge**: a second Attack action means Extra Attack applies again, and the Light property can trigger again (see E5).

---

## 4. Feats, backgrounds, species

### 4.1 Feat structure (2024)

Categories are **Origin** (level 1, from a background or the Human's Versatile trait), **General** (level 4+, nearly all +1 to an ability), **Fighting Style** (needs the Fighting Style feature), and **Epic Boon** (level 19+). Feats are taken once unless marked Repeatable [SRD p.87]. ASI: "+2 to one or +1 to two, max 20" [SRD p.87]. Every class gets **Epic Boon** at L19: "an Epic Boon feat ... or another feat of your choice for which you qualify" [SRD p.29].

| Feat | Category | Key 2024 mechanics (DPR-relevant) | SRD 5.2? | Source |
|---|---|---|---|---|
| **Savage Attacker** | Origin | "Once per turn when you hit a target with a weapon, you can roll the weapon's damage dice twice and use either roll." Weapon dice only [SAC-Feat-SavageAttacker1]. | **Yes** | SRD p.87 |
| Alert | Origin | +PB to Initiative; swap Initiative | Yes | SRD p.87 |
| Magic Initiate | Origin | 2 cantrips + one level 1 spell (Cleric, Druid or Wizard list) | Yes | SRD p.87 |
| Skilled | Origin | 3 skill or tool proficiencies | Yes | SRD p.87 |
| Tough, Lucky, Musician, Crafter, Healer, Tavern Brawler | Origin | Lucky: PB luck points for Advantage. Tavern Brawler: Unarmed Strike 1d4+Str, reroll 1s, push once per turn | **No** | 5ET:feats.json |
| **Great Weapon Master** | General (Str 13, +1 Str) | *Heavy Weapon Mastery* (paraphrased): on a hit with a Heavy weapon made as part of the Attack action on your own turn, you may add extra damage equal to your Proficiency Bonus. *Hew* (paraphrased): right after a Critical Hit with a Melee weapon, or after dropping a creature to 0 HP with one, you may make one attack with that same weapon as a Bonus Action. | No | 5ET:feats.json |
| **Sharpshooter** | General (Dex 13, +1 Dex) | Ignore Half and Three-Quarters Cover; no Disadvantage in melee or at long range. **No damage bonus.** | No | 5ET:feats.json |
| **Dual Wielder** | General (Str or Dex 13) | See §1.4 | No | 5ET:feats.json |
| **Charger** | General (Str or Dex 13) | Dash +10 ft. After moving ≥10 ft in a straight line immediately before a melee hit as part of the Attack action: +1d8 damage or a 10 ft push, once per turn | No | 5ET:feats.json |
| **Mage Slayer** | General (+1 Str or Dex) | Concentration Breaker (Disadvantage on the target's save); Guarded Mind (once per rest, turn a failed INT/WIS/CHA save into a success) | No | 5ET:feats.json |
| **Polearm Master** | General (Str or Dex 13) | *Pole Strike*: after the Attack action with a Quarterstaff, Spear, or a Heavy + Reach weapon, a Bonus Action attack (d4 Bludgeoning, **ability modifier added** [SAC-Feat-PolearmMaster1]). *Reactive Strike*: Reaction attack when a creature enters your reach. | No | 5ET:feats.json |
| **Crossbow Expert** | General (Dex 13, +1 Dex) | See §1.4 | No | 5ET:feats.json |
| Sentinel | General | Opportunity Attack when an adjacent creature Disengages or hits someone else; a hit sets Speed to 0 | No | 5ET:feats.json |
| Piercer / Slasher / Crusher | General (+1) | Piercer: reroll one damage die once per turn; +1 die on a crit. Crusher: on a crit, attacks against the target have Advantage. Slasher: on a crit, the target has Disadvantage | No | 5ET:feats.json |
| Fey-Touched | General (+1 Int/Wis/Cha) | Misty Step + a level 1 Divination or Enchantment spell (Hunter's Mark or Hex) | No | 5ET:feats.json |
| Grappler | General | Punch and Grab; Advantage against your grappled target | **Yes** | SRD p.87 |
| Weapon Master | General | +1 mastery kind | No | 5ET:feats.json |
| Archery / Defense / GWF / TWF | Fighting Style | Archery +2 ranged attack. GWF: "treat any 1 or 2 on a damage die as a 3" (Two-Handed or Versatile wielded in two hands; **not a reroll**) | **Yes** | SRD p.87–88 |
| Dueling / Thrown Weapon Fighting / Unarmed Fighting / Blind Fighting / Interception / Protection | Fighting Style | Dueling +2 damage (one-handed melee weapon, no other weapons). Thrown +2 damage on ranged Thrown hits. Unarmed 1d6 (1d8 with no weapon or shield) +Str | No | 5ET:feats.json |
| Blessed Warrior (Paladin) / Druidic Warrior (Ranger) | Class option in place of a style | Two Cleric or two Druid cantrips | No | 5ET:class-*.json |
| **Boon of Combat Prowess** | Epic Boon | *Peerless Aim*: "When you miss with an attack roll, you can hit instead. Once you use this benefit, you can't use it again until the start of your next turn." | **Yes** | SRD p.88 |
| **Boon of Irresistible Offense** | Epic Boon (+1 Str or Dex, max 30) | Bludgeoning, Piercing and Slashing damage ignores Resistance; on a natural 20, extra damage equal to the boosted ability **score** | **Yes** | SRD p.88 |
| Boon of Dimensional Travel, Fate, Spell Recall, Night Spirit, Truesight | Epic Boon | Mostly not DPR-relevant (Fate ±2d4 on a d20 test) | **Yes** | SRD p.88 |
| Boon of Speed, Fortitude, Recovery, Skill, Energy Resistance (and others) | Epic Boon | Not DPR-relevant | No | 5ET:feats.json |

### 4.2 Background → ability scores → Origin feat

"A background lists three of your character's ability scores. Increase one by 2 and another one by 1, or increase all three by 1. None of these increases can raise a score above 20." Each background also gives one specified Origin feat, two skills, one tool, and equipment or 50 GP [SRD p.83]. **2024 species give no ability score increases.**

| Background | Abilities | Origin feat | SRD? |
|---|---|---|---|
| Acolyte | Int/Wis/Cha | Magic Initiate (Cleric) | Yes |
| Criminal | Dex/Con/Int | Alert | Yes |
| Sage | Con/Int/Wis | Magic Initiate (Wizard) | Yes |
| Soldier | Str/Dex/Con | Savage Attacker | Yes |
| Artisan | Str/Dex/Int | Crafter | No |
| Charlatan | Dex/Con/Cha | Skilled | No |
| Entertainer | Str/Dex/Cha | Musician | No |
| Farmer | Str/Con/Wis | Tough | No |
| Guard | Str/Int/Wis | Alert | No |
| Guide | Dex/Con/Wis | Magic Initiate (Druid) | No |
| Hermit | Con/Wis/Cha | Healer | No |
| Merchant | Con/Int/Cha | Lucky | No |
| Noble | Str/Int/Cha | Skilled | No |
| Sailor | Str/Dex/Wis | Tavern Brawler | No |
| Scribe | Dex/Int/Wis | Skilled | No |
| Wayfarer | Dex/Wis/Cha | Lucky | No |

Source: 5ET:backgrounds.json, SRD p.83–84. **Solver implication:** the background is a joint choice of *(ability-score triple, Origin feat)*. A Str martial who wants Savage Attacker must be a Soldier. A Paladin who wants Str and Cha raised by the background has Noble (Skilled) or Entertainer (Musician). The **Human's Versatile** trait gives a second Origin feat of free choice, which decouples the two.

### 4.3 2024 species (all 10 are in the SRD [SRD p.84–86])

Aasimar, Dragonborn, Dwarf, Elf, Gnome, Goliath, Halfling, Human, Orc, Tiefling [5ET:races.json; SRD p.84–86]. DPR-relevant traits:
- **Goliath, Giant Ancestry** (uses = PB per long rest): *Fire's Burn* +1d10 Fire on a hit; *Hill's Tumble* knocks Prone on a hit (Large or smaller), which creates advantage; *Frost's Chill* +1d6 Cold and −10 Speed. *Large Form* at L5.
- **Aasimar, Celestial Revelation** (L3, 1 min, once per long rest, Bonus Action to transform): +PB damage once per turn.
- **Dragonborn, Breath Weapon**: replaces one attack of the Attack action.
- **Human, Versatile**: an extra Origin feat. **Halfling, Luck**: reroll natural 1s on d20 tests. **Orc, Adrenaline Rush**: Bonus Action Dash.
- **Elf**: Elven Lineage spells. **There is no Elven Accuracy in 2024**; that feat is XGE (2014-era).

---

## 5. MVP class chassis (2024)

Source: 5ET:class-*.json feature lists cross-checked with SRD class sections. The subclass comes at **L3 for every class**. In the SRD, the ASI feature means "Ability Score Improvement feat or another feat".

| Lvl | Fighter | Rogue | Barbarian | Ranger | Paladin | Monk |
|---|---|---|---|---|---|---|
| 1 | Fighting Style, Second Wind, Weapon Mastery (3) | Expertise, Sneak Attack 1d6, Thieves' Cant, Weapon Mastery (2) | Rage, Unarmored Defense, Weapon Mastery (2) | Spellcasting, Favored Enemy (Hunter's Mark), Weapon Mastery (2) | Lay on Hands, Spellcasting, Weapon Mastery (2) | Martial Arts (d6), Unarmored Defense |
| 2 | Action Surge, Tactical Mind | Cunning Action | Danger Sense, Reckless Attack | Deft Explorer, Fighting Style | Fighting Style, Paladin's Smite | Monk's Focus (Flurry, Patient Defense, Step of the Wind), Unarmored Movement, Uncanny Metabolism |
| 3 | Subclass | Subclass, Steady Aim | Subclass, Primal Knowledge | Subclass | Channel Divinity, Subclass | Deflect Attacks, Subclass |
| 4 | ASI (WM 4) | ASI | ASI (WM 3) | ASI | ASI | ASI, Slow Fall |
| 5 | Extra Attack, Tactical Shift | Cunning Strike, Uncanny Dodge | Extra Attack, Fast Movement | Extra Attack | Extra Attack, Faithful Steed | Extra Attack, Stunning Strike (MA d8) |
| 6 | ASI | Expertise | Subclass | Roving | Aura of Protection | Empowered Strikes, Subclass |
| 7 | Subclass | Evasion, Reliable Talent | Feral Instinct, Instinctive Pounce | Subclass | Subclass | Evasion |
| 8 | ASI | ASI | ASI | ASI | ASI | ASI |
| 9 | Indomitable, Tactical Master | Subclass | Brutal Strike (Rage dmg +3) | Expertise | Abjure Foes | Acrobatic Movement |
| 10 | Subclass (WM 5) | ASI | Subclass (WM 4) | Tireless | Aura of Courage | Heightened Focus (Flurry 3 strikes), Self-Restoration |
| 11 | Two Extra Attacks | Improved Cunning Strike | Relentless Rage | Subclass | Radiant Strikes (+1d8 Radiant on melee hits) | Subclass (MA d10) |
| 12 | ASI | ASI | ASI | ASI | ASI | ASI |
| 13 | Indomitable, Studied Attacks | Subclass | Improved Brutal Strike | Relentless Hunter | — | Deflect Energy |
| 14 | ASI | Devious Strikes | Subclass | Nature's Veil | Restoring Touch | Disciplined Survivor |
| 15 | Subclass | Slippery Mind | Persistent Rage | Subclass | Subclass | Perfect Focus |
| 16 | ASI (WM 6) | ASI | ASI (Rage dmg +4) | ASI | ASI | ASI |
| 17 | Action Surge ×2, Indomitable | Subclass | Improved Brutal Strike (2d10, two effects) | Precise Hunter | — | Subclass (MA d12) |
| 18 | Subclass | Elusive | Indomitable Might | Feral Senses | Aura Expansion | Superior Defense |
| 19 | Epic Boon | Epic Boon | Epic Boon | Epic Boon | Epic Boon | Epic Boon |
| 20 | Three Extra Attacks | Stroke of Luck | Primal Champion (+4 Str/Con, max 25) | Foe Slayer | Subclass | Body and Mind (+4 Dex/Wis, max 25) |

Rogue Sneak Attack is 1d6 per two Rogue levels, rounded up (1d6 at L1 to 10d6 at L19). Barbarian Rages: 2/2/3/3/3/4… up to 6. Rage damage +2 (L1), +3 (L9), +4 (L16). The Monk's Focus Points equal Monk level from L2.

**Key DPR features, verbatim or near-verbatim:**
- **Sneak Attack** [SRD p.61]: "Once per turn, you can deal an extra 1d6 damage to one creature you hit with an attack roll if you have Advantage on the roll and the attack uses a Finesse or a Ranged weapon ... You don't need Advantage ... if at least one of your allies is within 5 feet of the target, the ally doesn't have the Incapacitated condition, and you don't have Disadvantage." "Once per turn" covers **any** turn, so an off-turn Opportunity Attack can Sneak Attack [SAC-Rogue1].
- **Cunning Strike** [SRD p.63]: when dealing Sneak Attack damage, forgo dice for an effect ("remove the die before rolling"). L5: Poison (1d6), Trip (1d6), Withdraw (1d6). L11: two effects. L14: Daze (2d6), Knock Out (6d6), Obscure (3d6). DC = 8 + Dex + PB. *Trip* (Prone on a failed Dex save) is an **advantage source for later attacks**.
- **Steady Aim** (L3): a Bonus Action for Advantage on the next attack this turn, if you haven't moved; Speed becomes 0.
- **Rage** [SRD p.28]: a Bonus Action to enter; damage bonus "When you make an attack using Strength—with either a weapon or an Unarmed Strike—and deal damage". Resistance to Bludgeoning, Piercing and Slashing. No spells or Concentration. Lasts until the end of your next turn, extended by an attack roll against an enemy, forcing a save, or a Bonus Action. 10 minutes maximum.
- **Reckless Attack**: decided at the first attack roll; Advantage on Str-based attack rolls until the start of your next turn; attack rolls against you have Advantage [SAC-Barbarian1].
- **Brutal Strike** [SRD p.29]: "If you use Reckless Attack, you can forgo any Advantage on one Strength-based attack roll of your choice on your turn. The chosen attack roll mustn't have Disadvantage. If the chosen attack roll hits, the target takes an extra 1d10 damage..." plus Forceful Blow or Hamstring Blow. L13 adds Staggering and Sundering Blow. L17: 2d10 and two effects.
- **Berserker Frenzy** (SRD subclass, L3): while Raging and using Reckless Attack, the first target you hit with a Str attack each turn takes extra damage of (Rage Damage bonus)d6 [SRD p.30].
- **Martial Arts** [SRD p.50]: Monk weapons are Simple Melee weapons and Martial Melee weapons with Light. The benefits need you to be unarmed or wielding only Monk weapons, with no armor or shield. *Bonus Unarmed Strike* (a Bonus Action), *Martial Arts Die* (d6 → d12), *Dexterous Attacks*. This does **not** grant the Finesse property [SAC], so a Monk weapon without Finesse cannot Sneak Attack.
- **Flurry of Blows**: 1 Focus Point for two Unarmed Strikes as a Bonus Action (three from L10). **Stunning Strike**: once per turn, 1 Focus Point; a successful save still halves Speed and gives the next attack against the target Advantage [SRD p.51].
- **Champion** (SRD subclass): Improved Critical 19–20 (L3), Superior Critical 18–20 (L15) [SRD p.49].
- **Oath of Devotion** (SRD subclass): Sacred Weapon, using Channel Divinity as part of the Attack action, adds Cha to attack rolls for 10 minutes [SRD p.56].
- **Paladin Radiant Strikes** (L11): +1d8 Radiant on every melee weapon or Unarmed Strike hit [SRD p.55].

**SRD subclasses** (one per class): Berserker, Champion, Oath of Devotion, Hunter, Thief, Warrior of the Open Hand [SRD contents p.30, 49, 52, 56, 59, 64]. Every other 2024 subclass is PHB-only.

### 5.1 Multiclassing (2024) [SRD p.24–25]

- **Prerequisites**: "you must have a score of at least 13 in the primary ability of the new class and your current classes." Primary abilities: Barbarian Str; Fighter **Str or Dex**; Monk **Dex and Wis**; Paladin **Str and Cha**; Ranger **Dex and Wis**; Rogue Dex [SRD class tables].
- **What you gain**: only some starting proficiencies (e.g., Barbarian: hit die, Martial weapons, Shields; Fighter: Martial weapons, Light and Medium armor, Shields), **plus the class's level 1 features**, which includes Weapon Mastery [SRD p.28, 47]. You gain no extra saving-throw proficiencies [SAC].
- **Extra Attack does not stack** [SRD p.25]. Unarmored Defense: use only one AC calculation. Proficiency Bonus follows total level.
- **Spell slots**: full-caster levels + **half (round up)** of Paladin and Ranger levels, looked up on the Multiclass Spellcaster table [SRD p.25]. *Note*: the round-up means a Paladin 1 / Ranger 1 counts as caster level 2.
- **How multiple Weapon Mastery features combine** is not stated (see E17).

---

## 6. 2014 contrasts relevant to sandbox isolation

| Mechanic | 2014 text (source) | 2024 counterpart | Why it must be edition-gated |
|---|---|---|---|
| **GWM** | (paraphrased) Before a melee attack with a proficient heavy weapon, you may opt into −5 to hit for +10 damage on a hit. Bonus Action melee weapon attack on a crit or a kill (5ET:feats.json PHB) | +PB on Heavy hits as part of the Attack action; Hew | The -5/+10 policy depends on AC; 2024 has no such policy decision. |
| **Sharpshooter** | Long range, Cover, and the same -5/+10 for ranged weapons you are proficient with (PHB) | No damage component | Same as GWM. |
| **Elven Accuracy** (XGE) | (paraphrased) With advantage on a Dex-, Int-, Wis- or Cha-based attack roll, you may reroll one of the two dice once. Elf or half-elf prerequisite. Optimal play is equivalent to the best of 3d20 | Not reprinted | Changes crit and hit math (P(crit) at 20 = 1−0.95³ ≈ 14.3%). |
| **Polearm Master** | Bonus Action butt-end attack (d4), restricted to glaive, halberd, quarterstaff or spear; Opportunity Attack when creatures *enter* reach (PHB; pike only for the OA clause) | Pole Strike / Reactive Strike | **2014 PAM + GWM**: the butt-end attack is a heavy-weapon melee attack, so -5/+10 applies. In 2024 Pole Strike is a Bonus Action, not part of the Attack action, so the GWM PB damage does **not** apply. |
| **Crossbow Expert** | (paraphrased) After attacking with a one-handed weapon during the Attack action, a Bonus Action attack with a held hand crossbow. (PHB) | Only adds the modifier to the Light extra attack; a second weapon is needed | One hand crossbow is enough in 2014. |
| **Divine Smite** | Class feature, not a spell: "when you hit a creature with a melee weapon attack, you can expend one spell slot ... 2d8 ... plus 1d8 for each spell level higher than 1st, to a maximum of 5d8 ... +1d8 if undead or fiend, to a maximum of 6d8" (5ET:class-paladin.json PHB) | Spell, Bonus Action, one per turn | 2014: no action cost; once per *hit* (several per turn, on reactions or Opportunity Attacks, usable while Raging since it isn't a spell); not limited by any slot-per-turn rule. |
| **Action Surge** | "On your turn, you can take one additional action." (PHB) | "except the Magic action" | 2014 allows two action-cast spells, including two leveled spells. |
| **Bonus Action spell rule** | "You can't cast another spell during the same turn, except for a cantrip with a casting time of 1 action." (PHB p.202, 5ET:book-phb.json) | One slot per turn | Different constraint shapes: 2014 restricts *spells* after a BA spell; 2024 restricts *slots*. Example: in 2014, Action Surge + two Fireballs is legal, but Hunter's Mark (BA) + Fireball is not. In 2024, a slotted Hunter's Mark + an action cantrip is legal. |
| **Two-weapon fighting** | "light melee weapon that you're holding in one hand ... bonus action to attack with a different light melee weapon that you're holding in the other hand" (PHB p.195) | Light property; drawing is allowed [SAC-Combat27]; Light ranged weapons work | 2014 needs both weapons in hand; it has no Nick. |
| **Critical hits** | "Roll all of the attack's damage dice twice ... If the attack involves other damage dice, such as from the rogue's Sneak Attack feature, you roll those dice twice as well." (PHB p.196) | Same principle [SRD p.16] | No difference. |
| **Object interaction** | "you could draw your weapon as part of the same action you use to attack" (PHB p.190) | Per-attack equip token + a free interaction | 2014 has one free interaction only, so swap chains are much tighter. |
| **Savage Attacker** | (paraphrased) Once per turn, reroll a melee weapon attack's weapon damage dice and keep either total. (PHB) | Any weapon hit, Origin feat | Same name with a different trigger and scope. |
| **Hunter's Mark** | "extra 1d6 damage ... whenever you hit it with a weapon attack"; move "on a subsequent turn" (PHB) | Any attack roll; Force; no subsequent-turn limit | Same name with a different scope. |

---

## 7. Spec fact-check table

| # | Spec claim (location) | Verdict | Citation | Corrected wording |
|---|---|---|---|---|
| 1 | "Nick: Off-hand attack integrated into the Main Attack action, freeing the Bonus Action." (§2 P0) | **Imprecise** | SRD p.89–90; DDB-WM | "Nick: the Light property's extra attack (made with a *different* Light weapon, no ability modifier to damage without the TWF style) can be made as part of the Attack action instead of as a Bonus Action, once per turn. It needs the Attack action and an attack with a Light weapon. It adds no attack, but frees the Bonus Action." Drop the term "off-hand": 2024 has no off-hand concept. |
| 2 | "Vex: Advantage loops across sequential attacks against the same target." | **Imprecise** | SRD p.90 | "Vex: a hit that deals damage gives Advantage on your *next* attack roll against that creature before the end of your next turn. The advantage is consumed by that roll whatever its result. The chain continues only while Vex attacks keep hitting, and it does not stack with other Advantage sources." |
| 3 | "Graze: Guaranteed floor damage equal to ability modifier on missed attack rolls." | **Imprecise** | SRD p.90 | "Graze (Glaive and Greatsword only, mastery required): on a miss, deal damage equal to the attack's ability modifier, of the weapon's type. It cannot be increased by anything except the modifier (no magic-weapon bonus, Rage, GWM or riders). Resistance applies. A modifier ≤ 0 deals no damage." |
| 4 | "Topple, Cleave, Push, Sap, and Slow: Positional, action-denial, and multi-target flow modeling." | **Imprecise** | SRD p.90; Prone p.~186 | Topple is an **advantage generator** (Prone gives Advantage to attackers within 5 ft) and matters for single-target DPR. Sap reduces enemy offense. Push and Slow are positional. Cleave is multi-target (outside MVP single-target scope). |
| 5 | "reworked Great Weapon Master scaling via Proficiency Bonus without attack penalties" | **Correct** (incomplete) | 5ET:feats.json | Add: only on hits with a **Heavy** weapon (including Longbow and Heavy Crossbow) **as part of the Attack action on your turn**. Includes +1 Str and Hew (a Bonus Action attack after a crit or kill with a melee weapon). Hew, Pole Strike and Opportunity Attacks don't get the PB damage. |
| 6 | "updated Sharpshooter, Dual Wielder, Charger, and Mage Slayer" | **Correct** | 5ET:feats.json | Note: Sharpshooter has **no damage bonus**; Mage Slayer has **no reaction attack**; Charger is +1d8 once per turn and needs a ≥10 ft straight-line move. All are PHB-only (not SRD). |
| 7 | "Divine Smite and related spell-smites reclassified as Bonus Actions and spell slot consumption." | **Imprecise** | SRD p.125, 105, ~54 | "Divine Smite is now a level 1 spell cast as a Bonus Action immediately after hitting with a Melee weapon or Unarmed Strike. It is subject to the one-slot-per-turn rule, with one slot-free cast per long rest (Paladin's Smite). The smite spells were already Bonus Action spells in 2014; in 2024 they are cast *after* the hit, and several no longer need Concentration." |
| 8 | "Action Surge explicitly restricting the Magic action." | **Correct** | SRD p.48 | "Action Surge: one additional action, except the Magic action (which blocks action-time spells, including cantrips, and Magic-action items). Twice per rest from L17, only once per turn." |
| 9 | "One-spell-slot-per-turn limits rigorously enforced across main actions, bonus actions, and reactions." | **Correct** | SRD p.105; SAC-SpellCastingTime1/2 | Add: it applies on **every** turn (yours and others'). It counts slot **expenditure**: free casts and cantrips are not limited. |
| 10 | "updated 2024 subclass availability at Level 3" | **Correct** | 5ET:class-*.json | — |
| 11 | "standardized origin traits" | **Imprecise** | SRD p.83 | "Ability score increases moved from species to background (+2/+1 or +1/+1/+1 among the background's three listed abilities, max 20). Each background grants a fixed Origin feat. Species grant traits only." |
| 12 | 2014 "Power-Attack Meta (-5/+10)" via SS and GWM | **Correct** | 5ET:feats.json (PHB) | Note: -5/+10 applies only with proficiency; for GWM, only on melee attacks with heavy weapons. |
| 13 | "Elven Accuracy (triple-advantage)" | **Correct** (imprecise scope) | 5ET:feats.json (XGE) | Only on attack rolls using **Dex, Int, Wis or Cha** with advantage; Str attacks don't benefit. XGE feat, Elf/Half-Elf prerequisite. |
| 14 | "expanded critical hit ranges (e.g., Champion Fighter, Hexblade's Curse)" (2014 sandbox) | **Correct**, but the framing is misleading | SRD p.49 | The 2024 Champion also has 19–20 (L3) and 18–20 (L15). Crit-range expansion is **not 2014-only**; only Hexblade's Curse is 2014 (XGE). |
| 15 | "Polearm Master + Great Weapon Master bonus attacks" (2014) | **Correct** | 5ET:feats.json (PHB) | In 2014 the PAM butt-end attack can take -5/+10. In 2024 PAM + GWM's PB damage does **not** combine on Pole Strike. |
| 16 | "Crossbow Expert hand-crossbow bonus attacks" (2014) | **Correct** | 5ET:feats.json (PHB) | — |
| 17 | "double spellcasting via 2014 Action Surge" | **Correct** | 5ET:class-fighter.json (PHB); PHB p.202 | Allowed unless one of the spells was a Bonus Action spell (2014 BA-spell rule). |
| 18 | "free-action Divine Smites applied to critical hits" (presented as 2014-only) | **Imprecise** | 5ET:class-paladin.json (PHB); SRD p.125, p.16 | 2014 Divine Smite uses **no action** ("free action" is not a 5e term). It can be used on every hit, including reaction attacks, with no per-turn slot limit. 2024 smites are *also* chosen after seeing a crit, and their dice also double; the difference is once per turn, Bonus Action, own turn only. |
| 19 | Concentration: "Hex vs. Hunter's Mark vs. Spirit Guardians conflicts" | **Correct** | SRD p.179 | Add: Rage blocks all Concentration and spellcasting. |
| 20 | "2024 Divine Smite vs. Telekinetic Shove vs. Off-hand attacks" compete for the BA | **Correct** (terminology) | SRD p.~176; 5ET:feats.json (Telekinetic) | Replace "Off-hand attacks" with "Light-property extra attack (unless Nick moves it into the Attack action)". Add Hunter's Mark, Rage, Flurry, Cunning Action, Steady Aim, Hew, Pole Strike and the Dual Wielder attack. |
| 21 | "Object Interactions & Weapon Drawing: Models weapon stow/draw limits" | **Correct** | SRD p.177, p.13 | Specify: one equip *or* unequip per Attack-action attack, plus one free object interaction per turn, plus Quick Draw and Thrown draws. |
| 22 | Triggers: "Booming Blade secondary damage triggers" | **Imprecise (edition scope)** | 5ET:gendata-spell-source-lookup.json | Booming Blade is SCAG/TCE (2014-era). It is not in the 2024 PHB or SRD 5.2 and is on no MVP class list. |
| 23 | Triggers: "Great Weapon Master cleave conditions" | **Imprecise** | 5ET:feats.json | In 2024 this is **Hew** (Bonus Action attack after a crit or a reduction to 0 HP with a melee weapon). Avoid "cleave", which collides with the Cleave mastery. |
| 24 | Loadout example: "Attack 1 with Vex Dagger, attack 2 with Nick Scimitar" | **Wrong** | SRD p.91 | The Dagger's mastery is **Nick**, not Vex. Correct example: "Attack 1 with Vex Shortsword (Light) → Nick extra attack with Scimitar (or Dagger)". The Nick attack is the Light extra attack, not "attack 2" of Extra Attack. |
| 25 | "Magic Resistance on targets (calculating adv/disadv on saving throws)" | **Imprecise** | SRD Monster section (Magic Resistance) | Magic Resistance grants **Advantage** on saves against spells and other magical effects. It never imposes Disadvantage. *Exact monster wording not quoted here; check.* |
| 26 | MVP riders: "Hunter's Mark, Hex, Divine Smite, Spirit Shroud" for Fighter/Rogue/Barbarian/Ranger/Paladin/Monk | **Imprecise** | SRD p.140–141; 5ET:spells-tce.json; 5ET:feats.json | Hex is Warlock-only, reachable in the roster only via Fey-Touched (PHB feat) or a Warlock multiclass. Spirit Shroud is a TCE spell on the Paladin list only via TCE's optional expanded list (Paladin 9+). Barbarians can't use any of them while Raging. |
| 27 | "Standardized Baselines: 27-point buy ... toggles for +1, +2, +3 weapon" | **Correct** | — | Note: a +X weapon bonus does **not** increase Graze damage. |
| 28 | "opening with Vex to generate advantage before triggering a Nick extra attack" (§5.2) | **Correct** | SRD p.89–90 | It needs a Vex weapon with the Light property (Shortsword, Handaxe, Hand Crossbow) to trigger the Light extra attack, or a Light weapon used in one of the Attack-action attacks, plus mastery of both weapon kinds. |
| 29 | Sandbox: "Cross-edition rule pollution ... explicitly prohibited" | **Correct as a design choice; stricter than official guidance** | COMPAT | Officially, 2014-era content not reprinted in 2024 (e.g., TCE subclasses and spells) is usable with 2024 rules. Isolation needs an explicit "legacy-compatible content" whitelist; see ADR-3. |
| 30 | "Mechanical Data Source: Complete 5e.tools Corpus" | **Out of scope here** | — | Its rules text matched SRD 5.2.1 everywhere I checked. Licensing and provenance belong to another research item. |
| 31 | Implied: all six MVP classes use Weapon Mastery | **Imprecise** | 5ET:class-monk.json | The **2024 Monk has no Weapon Mastery** (only via the Weapon Master feat or multiclassing). |
| 32 | Smooth leveling / ASIs implied at standard levels | **Note** | 5ET:class-*.json | Fighter gets extra ASIs at 6 and 14; Rogue at 10. Every class gets an Epic Boon at 19. |

---

## 8. Edge-case register

Each case lists the ruling options. **[R]** marks the recommended default; the others should be available as toggles.

| ID | Edge case | Options | Basis |
|---|---|---|---|
| E1 | **Crit doubling of smite dice** (Divine Smite, smite spells, Radiant Strikes) | **[R]** A: double. "The target takes an extra 2d8 Radiant damage **from the attack**", and crits double "other damage dice" of the attack. B: don't double. | SRD p.16, p.125 |
| E2 | **Crit doubling of Hunter's Mark / Hex / Spirit Shroud / Colossus Slayer / Goliath Fire's Burn dice** | **[R]** A: double (dice delivered by the attack's hit; consistent with the SRD "other damage dice" sentence). B: double only when the text says "from the attack". | SRD p.16 (no explicit 2024 SAC ruling found; **unverified**) |
| E3 | **Graze and riders** | **[R]** A: Graze = ability modifier only. No on-hit riders (Sneak Attack, Hunter's Mark, smite, Rage, GWM, Savage Attacker), no +X weapon bonus. Resistance applies. A modifier ≤ 0 deals 0. B: allow Rage damage ("deal damage" with a Str attack). Contradicted by "can be increased only by increasing the ability modifier". | SRD p.90 |
| E4 | **Which weapon must carry Nick?** | **[R]** A: the weapon making the Light extra attack must have Nick (the conventional reading of a weapon's mastery property). B: either weapon. Nick lacks "with this weapon" wording, and community discussion is split. | SRD p.90; D&D Beyond forum threads (non-official) |
| E5 | **Nick + Action Surge (two Attack actions)** | **[R]** A: Light can trigger once per Attack action, so Nick attack (first action) + Light extra attack as a Bonus Action (second action) is allowed. B: only one Light extra attack per turn. | SRD p.89 (trigger "When you take the Attack action"); no official ruling found |
| E6 | **Dual Wielder attack + Two-Weapon Fighting style** | A: TWF adds the modifier ("extra attack as a result of using a weapon that has the Light property"). **[R]** B: it does not (the Dual Wielder attack comes from the feat, not the property). | SRD p.88; 5ET:feats.json. Ambiguous |
| E7 | **Does the Nick attack (or the Cleave attack) grant an equip/unequip token?** | **[R]** A: Nick yes ("as part of the Attack action"), Cleave no. B: neither. C: both. | SRD p.177, p.90 |
| E8 | **Sneak Attack once per turn, including off-turn reactions** | **[R]** Per-turn counter reset at the start of *every* creature's turn. Opportunity Attacks or readied attacks on other turns can Sneak Attack again. | SAC-Rogue1 |
| E9 | **Smite on Opportunity Attacks or reaction attacks** | **[R]** Not allowed (Bonus Action casting time; no Bonus Action off-turn). | SRD p.125; Bonus Action glossary |
| E10 | **Nick when the triggering attack is a Bonus Action** (Hew, Pole Strike, Monk bonus Unarmed Strike, Dual Wielder attack) | **[R]** No Light trigger, so no Nick (Light needs "the Attack action"). | SRD p.89 |
| E11 | **GWM PB damage on:** Hew, Pole Strike, Opportunity Attack, Cleave extra attack, Heavy ranged weapons | **[R]** Hew no, Pole Strike no, OA no; Cleave attack ambiguous (default no); Longbow and Heavy Crossbow **yes**. | 5ET:feats.json |
| E12 | **Vex consumption** | **[R]** Consumed by the next attack roll against that creature, even if that roll already had Advantage; refreshed by each Vex hit; expires at the end of your next turn. | SRD p.90 |
| E13 | **Topple sequencing** | **[R]** The engine chooses attack order: Topple first, then other attacks at Advantage within 5 ft; ranged attacks against Prone have Disadvantage. Prone persists until the target stands (half its movement, on its turn), so it doesn't carry over in sustained mode unless the target's Speed is 0. Needs the target's CON save. | SRD p.90, Prone |
| E14 | **Savage Attacker with crits and policy** | **[R]** Roll the (doubled) weapon dice twice and keep the higher total (expected value = E[max of two]); weapon dice only. Once per turn, choose which hit; the default is to apply it to a crit. | SRD p.87; SAC-Feat-SavageAttacker1 |
| E15 | **Rage + spells** | **[R]** Raging blocks Divine Smite, smite spells and Hunter's Mark casting, and ends Concentration. Free casts are blocked too. | SRD p.28 |
| E16 | **Brutal Strike vs advantage sources** | **[R]** Forgoing Advantage gives up all sources (Vex, Topple, Reckless Attack) on that roll. The engine chooses the attack; it must not have Disadvantage. The Vex charge is still consumed by that roll (it is "your next attack roll"). | SRD p.29 |
| E17 | **Multiple Weapon Mastery features (multiclass)** | **[R]** A: each feature grants its own kinds (union, total = sum). B: only the highest count. | SRD p.24–25 (silent) |
| E18 | **Free casts vs the one-slot rule** | **[R]** Exempt (no slot expended). | SRD p.105 |
| E19 | **Hunter's Mark move timing** | **[R]** 2024: Bonus Action, any later point including the same turn the target drops (no "subsequent turn" clause). Hex: "on a later turn". | SRD p.141, p.140 |
| E20 | **Boon of Combat Prowess policy** | **[R]** Converts the first miss on each of your turns into a hit (not a crit). The engine should apply it to the miss with the highest marginal value (e.g., a smite-eligible or Sneak Attack-eligible attack). Note that the reset is "start of your next turn", so it can apply off-turn. | SRD p.88 |
| E21 | **Advantage stacking** | **[R]** Binary: any Advantage plus any Disadvantage = neither; multiple sources don't stack. Elven Accuracy (2014 only) = 3 dice. | SRD p.176 |
| E22 | **Monk Flurry and weapon riders** | **[R]** Unarmed Strikes are not weapons: no mastery, no Sneak Attack (needs a Finesse or Ranged weapon), no Savage Attacker. Divine Smite and Radiant Strikes **do** work with Unarmed Strikes. | SRD p.177 (Unarmed Strike), p.125 |
| E23 | **Loading weapons without Crossbow Expert** | **[R]** One shot per action, Bonus Action or Reaction, so Extra Attack is wasted on Loading weapons. | SRD p.89 |
| E24 | **Dropped weapons in sustained loops** | **[R]** A dropped weapon must be re-equipped (one token) before reuse; stowing keeps it available. | SRD p.177 |
| E25 | **Charger in a single-target dummy model** | **[R]** Off by default; toggle "can move ≥10 ft straight before the first melee hit" (once per turn). | 5ET:feats.json |
| E26 | **Studied Attacks + Vex + Precise Hunter** | **[R]** These are all just Advantage sources; non-stacking. | SRD p.48, 59 |
| E27 | **Paladin's Smite free cast while also casting a slotted spell** | **[R]** Legal (e.g., slotted Shining Smite one turn; or a Magic-action slotted spell plus the free Divine Smite as a Bonus Action). | SRD p.105, ~54 |
| E28 | **2014 Divine Smite while Raging (sandbox)** | **[R]** Allowed in 2014 (it is a feature, not a spell); blocked in 2024. | 5ET:class-paladin.json (PHB); SRD p.28 |

---

## 9. Open questions

1. **Content scope for "2024 only":** is it XPHB/SRD-only, or XPHB plus non-reprinted 2014-era supplements (TCE Spirit Shroud, Booming Blade, XGE/TCE subclasses), as [COMPAT] permits? The answer decides whether Spirit Shroud (spec MVP) and Booming Blade stay. See ADR-3.
2. **Subclasses in MVP:** the SRD has one per class. Battle Master, Gloom Stalker, Vengeance, Zealot, Soulknife and others are PHB-only. Which ones are in scope, and on which text basis (licensing is outside this topic)?
3. **The Monk without Weapon Mastery:** should the solver consider the Weapon Master feat and multiclass dips for Monks? This is legal, but it affects the search space.
4. **Ambiguity defaults** (E2, E4, E5, E6, E7, E17): there is no official 2024 Sage Advice on these. Should defaults be conservative (lower DPR) or popular-consensus? Do we surface the DPR delta between rulings?
5. **Errata drift:** the D&D Beyond PHB errata page index reports "Last Updated: Oct 7, 2026", but the only items visible were from April 2025. Community reports say WotC now applies "stealth errata" directly in D&D Beyond. We need a process to diff SRD and DDB text periodically (**unverified**).
6. **Enemy model inputs** needed by these rules: CON save bonus (Topple), Str/Dex saves (Cunning Strike Trip, Thunderous Smite), WIS saves (Wrathful and Staggering Smite), creature type (Fiend/Undead for +1d8 smite), size (Push, Hill's Tumble, Topple has no size limit), and an "ally adjacent" toggle (Sneak Attack without Advantage).
7. **Sustained-DPR resource model:** short rests per long rest govern Action Surge, Rage, Focus Points, Channel Divinity and Paladin's Smite. The spec mentions "rest frequencies" without defaults.
8. The concentration flags on the PHB smite spells come from 5e.tools duration data; spot-check them against the PHB text.

---

## 10. Candidate ADRs

### ADR-1: Ambiguous rules are adjudicated as named, versioned Rulings, and every result is stamped with the active ruling set
- **Decision:** Every textually ambiguous mechanic (§8) becomes a named **Ruling** with a stable id (e.g., `R-NICK-WEAPON`), the options, a default, citations, and the rationale. The engine reads rulings from configuration; no interpretation is hard-coded in feature logic. Every DPR result, build ranking and combat script records the ruling-set version, and the dashboard can show the DPR delta of flipping a ruling.
- **Rationale:** Optimization results are only as credible as their rules assumptions. At least ten DPR-relevant ambiguities have no official answer, and Sage Advice changes over time. Hard-coded choices would make old rankings silently irreproducible and would be very costly to unpick later.
- **Rejected:** (a) one hard-coded interpretation per mechanic, which is opaque and can't be reversed without re-auditing code; (b) "whatever 5e.tools or community consensus says", which is not authoritative and still implicit; (c) free-form per-user house rules, which break comparability and search caching.

### ADR-2: Source-of-truth precedence for rules text
- **Decision:** Precedence is official errata, then SRD 5.2.x text, then PHB 2024 text (transcribed via 5e.tools only as a cross-check source, flagged), then Sage Advice (used as the *default ruling*, not as law), then community consensus (allowed only as a non-default ruling option). Each encoded feature stores its citation and source tier.
- **Rationale:** The SRD has an explicit license and already includes the errata. PHB-only content (GWM, Sharpshooter, most subclasses) is unavoidable for a useful optimizer, so it needs a clearly marked lower-trust tier. Sage Advice is explicitly a FAQ.
- **Rejected:** (a) 5e.tools as the primary truth, which is a community transcription with unclear provenance; (b) SRD-only, which drops GWM, Sharpshooter, Dual Wielder and all but one subclass per class.

### ADR-3: Ruleset isolation uses an explicit "legacy-compatible" content whitelist instead of edition-by-publication-date
- **Decision:** A ruleset is defined by its rules *chassis* (2024 or 2014 core rules, glossary and action economy). Content items carry an edition tag and, for 2014-era items not reprinted in 2024 (e.g., Spirit Shroud, Booming Blade, older subclasses), an opt-in `legacy-compatible-2024` flag. The 2024 sandbox loads 2024 core plus whitelisted legacy items, re-interpreted under 2024 chassis rules (one-slot-per-turn, the 2024 Attack action, and so on). Items reprinted in 2024 are always replaced by their 2024 version.
- **Rationale:** The spec's MVP already names Spirit Shroud and Booming Blade, which are 2014-era, while demanding strict isolation. The official guidance [COMPAT] lets non-reprinted older content run on 2024 rules. A per-item flag reconciles both and is hard to retrofit once the data model is built.
- **Rejected:** (a) strict XPHB/SRD-only, which removes spec-named riders; (b) allowing all 2014 supplements in 2024 mode, which imports reprinted-and-changed items and causes the "pollution" the spec forbids; (c) per-build user toggles only, which make results incomparable.

### ADR-4: Model the turn as a state machine with explicit hand state and per-attack equip tokens, not static "loadouts"
- **Decision:** Each attack event carries its source (Attack action / Light extra / Bonus Action / Reaction / Cleave …), which consumes and produces tokens (equip/unequip, free object interaction, Bonus Action, Reaction, slot-this-turn, once-per-turn flags keyed by turn owner). Weapon swaps are planned within this machine.
- **Rationale:** Masteries, Nick, Dual Wielder, GWM ("as part of the Attack action on your turn"), Sneak Attack (once per *any* turn) and smites (own-turn Bonus Action) all depend on the attack's *source* and *turn owner*. A static loadout model can't represent swap chains (§2.3) and would need a rewrite.
- **Rejected:** (a) static per-build loadouts with fixed attack lists, which are simple but can't express swaps or the per-source conditions; (b) full tactical simulation with positions, which is overkill for single-target MVP. Positional effects stay as toggles (Charger, ally adjacent).

---

## 11. Glossary candidates

| Term | Definition (one sentence) | Synonyms to avoid |
|---|---|---|
| **Ruleset** | The edition chassis (2024 or 2014) whose core rules govern an evaluation. | "edition mode", "5.5e", "One D&D" |
| **Weapon Mastery** | The class or feat feature that unlocks the mastery property of specific weapon *kinds* for a character. | "mastery slots" |
| **Mastery property** | The per-weapon property (Cleave, Graze, Nick, Push, Sap, Slow, Topple, Vex) usable only with Weapon Mastery for that kind. | "mastery", "weapon trait" |
| **Light extra attack** | The extra attack the Light property grants after an Attack-action attack with a Light weapon, made with a different Light weapon. | "off-hand attack", "TWF attack", "bonus attack" |
| **Nick attack** | A Light extra attack made as part of the Attack action through the Nick property. | "free off-hand", "extra Nick attack" |
| **Attack action** | The action whose attacks (including Extra Attack) each grant one equip/unequip. | "main attack", "Main Attack action" |
| **Equip token** | The single equip-or-unequip allowed per attack made as part of the Attack action. | "swap", "free draw" |
| **Free object interaction** | The one object interaction per turn that costs neither action nor movement. | "free action" |
| **Attack source** | The rule granting an attack (Attack action, Light, Nick, Bonus Action feature, Reaction, Cleave), which determines which riders apply. | — |
| **Rider** | Extra damage or an effect attached to a hit (Sneak Attack, Hunter's Mark, smite, Rage damage, GWM). | "proc", "on-hit bonus" |
| **Graze damage** | Damage equal to the attack's ability modifier dealt on a miss with a Graze weapon. | "miss floor", "chip damage" |
| **Slot-free cast** | Casting a leveled spell through a feature without expending a slot (e.g., Paladin's Smite, Favored Enemy), so the one-slot rule doesn't count it. | "free cast" (ambiguous), "free-action spell" |
| **One-slot rule** | The rule that only one spell slot may be expended to cast spells on any single turn. | "one spell per turn", "bonus action spell rule" (that is the 2014 rule) |
| **Hew** | The 2024 GWM Bonus Action attack after a crit or a drop to 0 HP with a melee weapon. | "GWM cleave", "GWM bonus attack" |
| **Ruling** | A versioned, cited choice among interpretations of ambiguous rules text, recorded with every result. | "house rule", "assumption" |
| **Origin feat** | A level 1 feat granted by a background (or the Human's Versatile trait). | "background feat", "1st-level feat" |
| **Epic Boon** | A level 19+ feat category granted by every class's L19 feature. | "capstone feat" |
| **Turn vs Round** | A turn belongs to one creature; a round is everyone's turns. Once-per-turn limits reset on every creature's turn. | using "round" for once-per-turn effects |
| **Sustained DPR / Nova DPR** | Expected damage per round over a multi-round encounter under a rest-budgeted resource plan / maximum expected damage over rounds 1–2 spending resources freely. | "white-room DPR", "burst" (undefined) |
