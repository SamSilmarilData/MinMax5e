---
status: accepted
---

# One turn state machine generates legal turns and validates any turn

> *Accepted 2026-11-27 (walking-skeleton lock, before any code).*

The turn is a state machine that tracks:

- what is in each hand;
- each attack's source (Attack action, Light extra attack, Bonus Action, Reaction, Cleave, and so on);
- per-attack equip/unequip tokens and the one free object interaction;
- Action, Bonus Action, and Reaction;
- whether a spell slot has been spent this turn;
- concentration;
- once-per-turn flags, keyed by whose turn it is.

A single pair of functions, `legalSteps` and `apply`, is the only authority on action-economy legality. The plan generator only follows `legalSteps`. Every plan, including ones a user edits, is replayed through `apply`, which returns a typed `Illegal(reason)` such as "Bonus Action already used by Divine Smite".

Masteries, Nick, Dual Wielder, Great Weapon Master ("as part of the Attack action on your turn"), Sneak Attack (once per *any* turn) and smites (a Bonus Action on your own turn) all depend on where an attack comes from and whose turn it is. A model with a fixed weapon loadout per build can't express swap chains. Having one authority provides the spec §5.2 guarantees, gives explanations when a plan is rejected, and allows a property test that generated plans are always legal.

## Considered Options

- Fixed weapon loadouts with a fixed attack list. Rejected: can't express swaps or the conditions that depend on an attack's source.
- Validate only. Rejected: wastes search effort and gives poor explanations.
- Construct only. Rejected: unverifiable, and offers no path for checking a user's edits.
- Full tactical simulation with positions. Rejected: overkill for the single-target MVP. Positional effects (Charger, an ally adjacent) stay as toggles.

Research: [02](../research/02-rules-mechanics-2024.md) §2, [04](../research/04-rules-engine-architecture.md)
