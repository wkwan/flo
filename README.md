# Bevy Fluid Sim
<video src="https://github.com/user-attachments/assets/ce8db8c1-f315-4d46-bf30-e0de866a0577" controls="controls" style="max-width: 730px;">
</video>
A barebones fluid simulator in Bevy (Rust/wgpu game engine) solving the shallow-water equations using the simplified pipe method. Rendering is done with WGSL raytracing. It's got lots of mistakes, but this is just one teeny feature in my game so I'll get back to it later!

## Run demo

```bash
cargo run
```

## References
### Simulation
https://lisyarus.github.io/blog/posts/simulating-water-over-terrain.html#section-virtual-pipes-method
https://github.com/akauper/Shallow-Water
### Rendering
https://www.shadertoy.com/view/MttfW8

## Misc
If you're curious about the long-winded commit history and the [CLAUDE.md](CLAUDE.md) file: I was trying to vibe code this with Claude Code, got confused, and then started coding myself and using more specific prompts. I still like Claude Code, just not for everything 😆
