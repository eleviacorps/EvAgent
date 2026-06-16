---
name: motion-design-patterns
domain: media
version: 1
trigger_patterns:
  - "motion graphics"
  - "animation patterns"
  - "kinetic typography"
  - "easing"
  - "transition design"
applicable_agents:
  - motion-designer
  - video-editor
---
# Motion Design Patterns

## Steps
1. Define animation purpose: inform (data viz), emphasize (title reveal), transition (scene change)
2. Choose easing: ease-out for objects entering (natural deceleration), ease-in-out for looping
3. Apply motion hierarchy: primary action first, secondary actions follow naturally
4. Use overlapping motion: stagger entrances so elements don't move simultaneously
5. Design transitions: cut, dissolve, wipe, morph — choose based on narrative pacing
6. Animate with intent: every movement should communicate meaning (scale for importance, position for focus)
7. Maintain consistent timing: 300ms for micro-interactions, 500ms for UI transitions, 1-2s for reveals

## Examples
- Kinetic typography: words animate in with the speaker's cadence, emphasis on key terms
- Data visualization: bars grow from bottom with ease-out, numbers count up, labels fade in
- Logo reveal: scale up with ease-out + slight overshoot, then subtle glow/particle accent
- Lower third: text slides in from left, small delay on secondary text, slides out on same curve

## Anti-patterns
- Linear animation (looks robotic — always use easing curves)
- Too many simultaneous movements (visual noise, hard to follow)
- Overly long animations (users wait for content)
- Motion without purpose (decorative animation distracts from the message)
- Inconsistent timing between elements (feels disjointed)
