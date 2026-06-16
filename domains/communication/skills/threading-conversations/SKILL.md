---
name: threading-conversations
domain: communication
version: 1
trigger_patterns:
  - "conversation thread"
  - "multi-turn"
  - "reply chain"
  - "context management"
applicable_agents:
  - messenger-bot
  - social-media-manager
  - email-composer
---
# Threading Conversations

## Steps
1. Track the conversation history: maintain context of previous messages and decisions
2. Acknowledge previous points before introducing new ones ("Following up on your question about...")
3. Quote or reference the specific message you're replying to when appropriate
4. Maintain coherence: each response should logically connect to the thread
5. Signal topic changes explicitly ("Switching gears..." / "On a different note...")
6. Close threads properly: summarize decisions, note next steps, or confirm resolution

## Examples
- Email thread: reference previous email subject, quote relevant section, add numbered points
- Slack thread: use thread replies to keep discussions organized, summarize long threads
- Customer support: acknowledge issue, restate solution, confirm resolution, ask for closing feedback

## Anti-patterns
- Replying without context ("Yes" — yes to what?)
- Changing the subject mid-thread without signaling
- Letting threads drift without resolution (orphaned conversations)
- Not quoting or referencing when there are multiple simultaneous threads
