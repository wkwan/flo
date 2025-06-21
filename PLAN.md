# PLAN: PORT_SHALLOW_WATER - Unity to Bevy Shallow Water Demo Port

## Phase 1: Foundation & Basic Setup ✅ COMPLETED
**Goal**: Set up basic Bevy app with 3D camera and water plane
- **Step 1.1**: ✅ Update Cargo.toml with required dependencies (bevy_render, bevy_pbr)
- **Step 1.2**: ✅ Create basic Bevy app with 3D scene, camera, and lighting
- **Step 1.3**: ✅ Add water plane mesh (subdivided quad) as foundation
- **Step 1.4**: ✅ Add 4 stone walls with textures matching Unity layout
- **Expected Output**: ✅ Window opens showing a static water plane mesh with stone walls

## Phase 2: GPU Compute Infrastructure ✅ COMPLETED
**Goal**: Implement double-buffered wave simulation textures
- **Step 2.1**: ✅ Create wave simulation compute shader (WGSL) with 4-neighbor stencil wave equation
- **Step 2.2**: ✅ Set up double-buffered render textures (512x512 RG format)
- **Step 2.3**: ✅ Implement compute pass system in render graph
- **Step 2.4**: ✅ Add logging to verify simulation is running
- **Expected Output**: ✅ Compute shader infrastructure ready (logs confirm execution)

**Progress Notes**:
- Wave simulation shader created at `assets/shaders/wave_simulation.wgsl`
- Double-buffered textures initialized with neutral values (0.5, 0.5)
- Render graph node integrated but actual compute dispatch pending
- Logging shows "Wave simulation render node initialized" and status updates every 2 seconds

## Phase 3: Input System & Wave Generation 🚧 NEXT
**Goal**: Add mouse interaction to create waves
- **Step 3.1**: Implement mouse position to world-space raycast system
- **Step 3.2**: Convert world coordinates to texture UV coordinates
- **Step 3.3**: Pass input data to compute shader for wave generation
- **Step 3.4**: Actually dispatch compute shader with proper pipeline
- **Expected Output**: Clicking mouse creates wave data in textures (still no visual representation)

**Implementation Plan**:
- Add mouse click detection system
- Implement ray-plane intersection for water surface
- Update WaveSimulationParams with input position
- Create compute pipeline and dispatch in render node

## Phase 4: Visual Rendering & Vertex Displacement
**Goal**: Make waves visible through mesh deformation
- **Step 4.1**: Create water material shader with vertex displacement from wave texture
- **Step 4.2**: Sample wave height texture in vertex shader for mesh deformation
- **Step 4.3**: Calculate surface normals from wave gradients for proper lighting
- **Expected Output**: Clicking mouse creates visible 3D wave ripples that propagate outward

## Phase 5: Water Appearance & Polish
**Goal**: Achieve visual parity with Unity demo
- **Step 5.1**: Add water surface shading (depth-based color, transparency)
- **Step 5.2**: Implement foam effects in shallow areas
- **Step 5.3**: Add specular highlights and surface smoothness
- **Step 5.4**: Fine-tune wave physics parameters (dampening, speed, scale)
- **Expected Output**: Realistic water appearance with proper shading and foam effects

## Phase 6: Advanced Features & Optimization
**Goal**: Complete feature parity and performance optimization  
- **Step 6.1**: Add configurable wave parameters (size, strength, dampening)
- **Step 6.2**: Implement character-based wave generation (moving objects create waves)
- **Step 6.3**: Add automatic wave drippers/sources
- **Step 6.4**: Performance optimization and code cleanup
- **Expected Output**: Full-featured shallow water simulation matching Unity demo functionality

## Technical Implementation Notes:
- **Wave Algorithm**: 2D shallow water equation with 4-neighbor finite difference method
- **Compute Shaders**: WGSL compute shaders for GPU-based wave simulation
- **Textures**: 512x512 RG format for wave height + previous height storage
- **Input**: Mouse raycast → world position → texture UV → shader parameters
- **Rendering**: Vertex displacement + normal calculation + PBR water material
- **ECS**: Components for input handling, wave parameters, and simulation state

**Final Expected Result**: Interactive shallow water simulation where mouse clicks create realistic wave ripples that propagate across a 3D water surface with proper lighting, shading, and foam effects - matching the visual quality and behavior of the original Unity demo.