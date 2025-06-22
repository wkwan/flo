# CLAUDE.md

## Claude Workflow

After editing code, run "cargo check" and fix compile errors. Once it "cargo check" succeeds, summarize changes, tell me what to expect when I test, and then stop. I will run the game manually.

## External Resources

Bevy documentation: https://docs.rs/bevy/latest/bevy/all.html
Bevy source code: https://github.com/bevyengine/bevy
bevy_egui documentation: https://docs.rs/bevy_egui/latest/bevy_egui
bevy_egui source code: https://github.com/vladbat00/bevy_egui 

## Coding Style

Stick to the most basic Rust/WGSL language features possible.
Avoid unneccessary software design patterns.
Use functions to avoid code duplication.
Don't make functions for logic that's only used once.
Put all numerical constants in src/constants.rs


