---
name: asset-management
domain: media
version: 1
trigger_patterns:
  - "asset organization"
  - "file naming"
  - "media management"
  - "version control"
  - "project organization"
applicable_agents:
  - motion-designer
  - video-editor
  - audio-engineer
---
# Media Asset Management

## Steps
1. Establish naming convention: ProjectName_AssetType_Description_Version (e.g., "CorpVideo_Logo_Animated_v02")
2. Organize folder structure: Project > Footage / Audio / Graphics / Exports / Assets
3. Use consistent file formats: ProRes/DNxHD for video, WAV/AIFF for audio, PNG/TIFF for graphics
4. Version files properly: save iterations with incremental numbers, never overwrite originals
5. Maintain a project bible: document assets used, sources, licenses, versions, and dates
6. Archive completed projects: include all assets, project files, exports, and notes
7. Back up regularly: local + cloud/offsite, verify backups periodically

## Examples
- Folder structure:
  ```
  ProjectName/
  ├── 00_Footage/
  ├── 01_Audio/
  │   ├── Voiceover/
  │   ├── Music/
  │   └── SFX/
  ├── 02_Graphics/
  │   ├── Logos/
  │   ├── Illustrations/
  │   └── Templates/
  ├── 03_Exports/
  │   ├── Drafts/
  │   └── Finals/
  └── 04_ProjectFiles/
  ```
- Naming: "ProductLaunch_HeroBG_Animated_v03.mov"

## Anti-patterns
- "final_final_v2_FINAL.mov" — meaningless versioning
- Mixing source footage with exports in the same folder
- No backup (hard drive failure = project loss)
- Missing license information for stock assets (legal risk)
- Deleting source files after export (can't make edits later)
