---
name: audio-mixing
domain: media
version: 1
trigger_patterns:
  - "audio mixing"
  - "sound levels"
  - "EQ"
  - "compression"
  - "audio mastering"
  - "voiceover levels"
applicable_agents:
  - audio-engineer
  - video-editor
---
# Audio Mixing

## Steps
1. Set levels: dialogue at -12dB to -6dB peaks, music at -18dB to -12dB, SFX at -12dB to -6dB
2. Apply EQ: high-pass filter (80Hz) for dialogue, cut muddiness (250-500Hz), add presence (2-5kHz)
3. Use compression: dialogue (ratio 2:1 to 4:1, threshold -20dB), music (gentle 2:1), SFX per need
4. Create spatial mix: pan dialogue center, music stereo, SFX to match on-screen position
5. Balance elements: dialogue should be clear and intelligible above music and SFX
6. Add ambience: room tone or background sound to fill silence (no dead air)
7. Master to standard: -14 LUFS for streaming (Spotify, YouTube), -16 LUFS for broadcast

## Examples
- Interview: dialogue center, gentle room tone underneath, subtle music bed at -20dB
- Explainer video: voiceover center, music -18dB, SFX for emphasis (ding, whoosh)
- Music video: stereo spread, kick and bass centered, vocals slightly above instrument mix
- Podcast: all speakers normalized to -12dB, compression to even out dynamics, noise gate on each mic

## Anti-patterns
- Clipping (peaking above 0dB — digital distortion)
- Muddy mix (too much low-mid buildup, lack of EQ)
- Dialogue buried under music or SFX
- No silence or room tone (abrupt cuts sound unnatural)
- Over-compression (sucking the life out of dynamics)
