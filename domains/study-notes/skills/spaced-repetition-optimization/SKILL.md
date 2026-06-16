---
name: spaced-repetition-optimization
domain: study-notes
version: 1
trigger_patterns:
  - "spaced repetition"
  - "SRS"
  - "Anki"
  - "memory schedule"
  - "review intervals"
applicable_agents:
  - flashcard-creator
  - note-synthesizer
---
# Spaced Repetition Optimization

## Steps
1. Format content for SRS systems (Anki, SuperMemo, Mnemosyne) with proper card types
2. Apply minimum information principle: one atomic fact per card
3. Design cloze deletions for lists and enumerations (one blank per card)
4. Use mnemonic techniques: imagery, acronyms, memory palaces for hard concepts
5. Set initial intervals: 1 min → 10 min → 1 day → 3 days → 1 week → 1 month
6. Tag cards by subject, difficulty, and confidence level for targeted review sessions
7. Regularly review leeches (cards you consistently get wrong) and reformat them

## Examples
- Atomic card: Q: "What is the time complexity of binary search?" A: "O(log n)"
- Cloze deletion: "The HTTP {{c1::status code}} for 'Not Found' is {{c2::404}}"
- Image occlusion: label parts of a diagram

## Anti-patterns
- Putting too much information on one card (defeats the purpose)
- Including cards you already know well without reviewing them
- Adding cards faster than you can review them (card bankruptcy)
- Using the same interval for all subjects (adjust based on difficulty)
