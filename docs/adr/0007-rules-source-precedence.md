---
status: proposed
---

# Order of trust between rules sources

When sources disagree, precedence is:

1. official errata;
2. SRD 5.2.x text;
3. PHB 2024 text, read from the local 5e.tools corpus (ADR-0027). It is a community transcription, so it is flagged as a lower-trust tier;
4. Sage Advice, used as the *default option* of a Ruling (ADR-0006) and not as binding text;
5. community consensus, allowed only as a non-default Ruling option.

Every encoded feature stores its citation and source tier. The SRD has an explicit licence and already includes errata. PHB-only content can't be avoided for a useful optimizer, so it needs a visible lower-trust tier. Sage Advice is officially a FAQ.

Research: [02](../research/02-rules-mechanics-2024.md)
