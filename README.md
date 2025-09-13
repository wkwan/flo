# Flo: Vulkan/Ash 3D Renderer Integrated with the Bevy Game Engine

## Warning

In it's current state, Flo isn't meant to be used out-of-the-box, so it's not published as a crate. If you're a gamedev who wants to use Rust + Vulkan for a 3D game, you can fork this repo, learn from it, and modify it for your use case.

PR's are much appreciated, but if not, please ⭐**star** the repo so I know you found it helpful!

## Motivation

I like Bevy but I've replaced the WGPU renderer with a Vulkan/ash renderer because:

- I want it fast and beautiful (hardware raytracing + lower CPU/GPU overhead)
- I don't care about exporting to web
- Ash is up-to-date with Vulkan, while WGPU is unstable and doesn't have the latest Vulkan features
- Tiny Glade is a successful example using Bevy + Vulkan/ash, no other comparable game using the default Bevy renderer

My [colony simulation game](https://www.youtube.com/watch?v=xsxvuzM5Oyg), which is the main use case for the engine, does lots of real-time procedural generation/animation and raytracing. I'm running into performance issues with my prototype and I've identified the renderer as the bottleneck. So I want to optimize the renderer to let players in my game do crazier things.

## Setup

1. Follow [Bevy setup instructions](https://bevy.org/learn/quick-start/getting-started/setup/) for Windows or Linux (this project is setup for Linux).

    If you choose not to install the alternative linker specified in [.cargo/config.toml](.cargo/config.toml) (mold for Linux, lld for Windows), then delete [.cargo/config.toml](.cargo/config.toml). This might make compilation much slower.

2. Install Vulkan SDK
3. Install glslc or glslangValidator to compile shaders (only needed if you're modifying the shaders)

#### For Windows Users Only:

Delete the [rust-toolchain.toml](rust-toolchain.toml) file. Also, if you installed the lld linker, keep [.cargo/config.toml](.cargo/config.toml) but delete all content except for the bottom 2 lines. 

```
[target.x86_64-pc-windows-msvc]
linker = "rust-lld.exe"
```

## Examples

You can watch these examples running on the Steam Deck in [this video](https://www.youtube.com/watch?v=y1m30oOksmI).  

Flo is simpler, uglier, and harder to to use than the wgpu renderer, so these benchmarks aren’t a fair comparison. However, the FPS loss from improving this project will partially be offset by making the pipeline multi-threaded. It's currently single-threaded for simplicity.

| Example | Command | Description | FPS Benchmark on Steam Deck LCD (averaged over 10s) |
|---------|---------|-------------| --------------------------------- |
| **Triangle** | `cargo run --release --example triangle` | Basic red triangle - Vulkan/ash integration, basic pipeline setup | 1820.1
| **Triangle (Bevy)** | `cargo run --release --example triangle_bevy` | Same triangle using Bevy's default WGPU renderer for performance comparison | 451.0
| **Cube** | `cargo run --release --example cube` | Animated 3D cube - directional lighting, push constants for rotation | 1817.7
| **Cube (Bevy)** | `cargo run --release --example cube_bevy` | Same cube using Bevy's default WGPU renderer for performance comparison | 445.9
| **Wireframe Cube** | `cargo run --release --example wireframe_cube` | Wireframe rendering - line-to-triangle conversion, rotating camera, custom mesh generation | 1769.4
| **Wireframe Cube (Bevy)** | `cargo run --release --example wireframe_cube_bevy` | Wireframe cube using Bevy's built-in wireframe plugin for performance comparison | 449.2
| **Grapes** | `cargo run --release --example grapes` | GLB model with textures - 1024x1024 texture, descriptor sets, 784 vertices/2064 indices | 1715.0
| **Grapes (Bevy)** | `cargo run --release --example grapes_bevy` | Same grapes model using Bevy's default WGPU renderer for performance comparison | 392.6
| **Grapes 1000** | `cargo run --release --example grapes_1000` | 1000 grapes instances - stress test for instanced rendering with Vulkan/ash | 351.9
| **Grapes 1000 (Bevy)** | `cargo run --release --example grapes_1000_bevy` | 1000 grapes instances using Bevy's default WGPU renderer for performance comparison | 73.9
| **Fluid Sim** | `cargo run --release --example fluid_sim` | Fluid simulation using unified renderer - dynamic mesh updates, multiple pipelines, interactive water physics | 520.1 (manually creating waves constantly throughout the test)
| **Fluid Sim (Bevy)** | `cargo run --release --example fluid_sim_bevy --features` | Fluid simulation using Bevy's default WGPU renderer with Bevy's full default features for performance comparison | 246.3 (manually creating waves constantly throughout the test)
| **Aula** | `cargo run --release --example aula` | Classroom scene with desks and chairs - complex multi-textured model, texture array rendering, depth testing | 1330.2 (manually moving and rotating camera constantly throughout the test)
| **Aula (Bevy)** | `cargo run --release --example aula_bevy` | Classroom scene using Bevy's default renderer - camera controls, GLTF loading, performance comparison | 261.4 (manually moving and rotating camera constantly throughout the test)
| **Mannequin Animation** | `cargo run --release --example mannequin_animation` | Skinned mesh animation - skeletal animation, joint transforms, GLTF animation playback | 1203.0
| **Mannequin Animation (Bevy)** | `cargo run --release --example mannequin_animation_bevy` | Same mannequin animation using Bevy's default WGPU renderer for performance comparison | 475.2
| **Egui** | `cargo run --release --example egui` | Interactive GUI with egui - UI overlays, mouse/keyboard input handling, multiple windows with widgets | 980.0
| **Egui (Bevy)** | `cargo run --release --example egui_bevy` | Same egui interface using bevy_egui integration for performance comparison | 402.7
| **GLB Inspector** | `cargo run --release --example inspect_glb assets/<modelname>.glb` | Analyze GLB files - texture formats, materials, mesh data, asset debugging tool |

## Modifying GLSL Shaders

When running an example after changing a GLSL shader, compile the shaders first:
- **Linux**: `./compile_shaders.sh`
- **Windows**: `powershell -ExecutionPolicy Bypass -File compile_shaders.ps1`

## Vulkan Rendering Architecture

### Unified Pipeline Design

This renderer uses a **unified pipeline approach** where a single graphics pipeline handles multiple object types through runtime configuration rather than switching between specialized pipelines.

```text
┌─────────────────────────────────────────────────────────────┐
│                      Game Application                       │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                         Bevy ECS                            │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                 VulkanRenderer (UNIFIED)                    │
│               (vulkan_renderer_unified.rs)                  │
├─────────────────────────────────────────────────────────────┤
│  SINGLE PIPELINE APPROACH                                   │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Created ONCE during initialization                 │    |
│  │  Bound ONCE per frame to command buffer             │    |
│  │  • Handles textured meshes                          │    │
│  │  • Handles untextured meshes                        │    │
│  │  • Handles instanced rendering                      │    │
│  │  • Handles texture arrays                           │    │
│  │  • Runtime behavior via push constants/descriptors  │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                             │
│  Configuration Per Object:                                  │
│  ┌─────────────────────────────────────────────────────┐    │
│  │ • Push Constants → MVP matrices, instance data      │    │
│  │ • Descriptor Sets → Textures, samplers              │    │
│  │ • Vertex Buffers → Mesh geometry                    │    │
│  │ • Draw Commands → Index count, instance count       │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                      Vulkan/Ash Layer                       │
│                  (Low-level Vulkan Bindings)                │
├─────────────────────────────────────────────────────────────┤
│  Command Buffer Recording (Per Frame):                      │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  1. vkCmdBindPipeline() ← Bind ONCE per frame       │    │
│  │  2. For each object:                                │    │
│  │     • vkCmdPushConstants(transform_matrix)          │    │
│  │     • vkCmdBindDescriptorSets(textures)             │    │
│  │     • vkCmdBindVertexBuffers(mesh_data)             │    │
│  │     • vkCmdDrawIndexed(vertices, indices)           │    │
│  │  3. vkQueueSubmit() ← Execute all commands          │    │
│  │                                                     │    │
│  │  Pipeline Creation: ONCE during app startup         │    │
│  │  Pipeline Binding: ONCE per frame                   │    │
│  │  Object Rendering: Many times per frame             │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────┬───────────────────────────────────────────┘
                  │
┌─────────────────▼───────────────────────────────────────────┐
│                    GPU EXECUTION                            │
│           (NVIDIA RTX / AMD RDNA / Intel Arc)               │
├─────────────────────────────────────────────────────────────┤
│  PIPELINE RUNS CONTINUOUSLY (Assembly Line Model)           │
│                                                             │
│  ┌──────────────────────────┐  ┌─────────────────────────┐  │
│  │   Vertex Shader Stage    │→ │  Fragment Shader Stage  │  │
│  │  • Process vertices      │  │  • Render pixels        │  │
│  │  • Apply MVP matrices    │  │  • Sample textures      │  │
│  │  • Handle instancing     │  │  • Calculate lighting   │  │
│  │  • Different mesh types  │  │  • Different materials  │  │
│  │                          │  │                         │  │
│  │  Pipeline stays active   │  │  Continuous processing  │  │
│  │  while processing all    │  │  of different object    │  │
│  │  object types            │  │  types                  │  │
│  └──────────────────────────┘  └─────────────────────────┘  │
│                                                             │
│  Key Concept: Same shaders process different data           │
│  Objects differentiated by uniforms, not pipelines          │
└─────────────────────────────────────────────────────────────┘
```
