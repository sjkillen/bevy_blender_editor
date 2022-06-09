## Unusable State

# bevy_blender_editor
Use Blender as a Bevy editor

## Goals
- Build the mimimal set of tools required to use Blender as a game editor for games built with Bevy
- Opinionated, but small enough to be forked and rewritten to suit a workflow
- Properties can be changed while game is running, but no code or asset hot reloading
- No config files, just code
- Automatic recompilation

### Rust Code
- Macros to designate properties of structs as editable
- A server to receive property edits
- A "render to blender" mode
### Blender Addon
- Define a render engine that receives frames from game.
- Create UI panel for reading / writing object properties
