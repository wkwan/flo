#import bevy_pbr::{
    mesh_functions,
    pbr_functions::{alpha_discard, prepare_world_normal, apply_pbr_lighting},
    pbr_types::{PbrInput, pbr_input_new},
    mesh_vertex_output::MeshVertexOutput,
    mesh_view_bindings::view,
}

@group(2) @binding(0) var wave_texture: texture_2d<f32>;
@group(2) @binding(1) var wave_sampler: sampler;

struct WaterMaterialUniform {
    wave_amplitude: f32,
    color: vec4<f32>,
}

@group(2) @binding(2) var<uniform> uniform: WaterMaterialUniform;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) displaced_position: vec3<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    // Sample wave height from texture
    let wave_sample = textureSampleLevel(wave_texture, wave_sampler, vertex.uv, 0.0);
    let wave_height = (wave_sample.x - 0.5) * uniform.wave_amplitude;
    
    // Displace vertex position in Y (up)
    var displaced_position = vertex.position;
    displaced_position.y += wave_height;
    
    out.displaced_position = displaced_position;
    out.uv = vertex.uv;
    
    // Transform to world space
    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(displaced_position, 1.0));
    
    // Calculate normal from wave gradients for proper lighting
    let texel_size = 1.0 / 512.0; // Wave texture is 512x512
    
    // Sample neighboring heights for gradient calculation
    let height_right = textureSampleLevel(wave_texture, wave_sampler, vertex.uv + vec2<f32>(texel_size, 0.0), 0.0).x;
    let height_up = textureSampleLevel(wave_texture, wave_sampler, vertex.uv + vec2<f32>(0.0, texel_size), 0.0).x;
    let height_left = textureSampleLevel(wave_texture, wave_sampler, vertex.uv - vec2<f32>(texel_size, 0.0), 0.0).x;
    let height_down = textureSampleLevel(wave_texture, wave_sampler, vertex.uv - vec2<f32>(0.0, texel_size), 0.0).x;
    
    // Calculate gradients
    let gradient_x = (height_right - height_left) * uniform.wave_amplitude;
    let gradient_z = (height_up - height_down) * uniform.wave_amplitude;
    
    // Calculate normal from gradients
    let surface_normal = normalize(vec3<f32>(-gradient_x, 1.0, -gradient_z));
    out.world_normal = mesh_functions::mesh_normal_local_to_world(surface_normal, vertex.instance_index);
    
    out.clip_position = view.clip_from_world * out.world_position;
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    var pbr_input = pbr_input_new();
    
    pbr_input.material.base_color = uniform.color;
    pbr_input.material.perceptual_roughness = 0.1;
    pbr_input.material.metallic = 0.0;
    pbr_input.material.alpha_cutoff = 0.5;
    
    pbr_input.frag_coord = in.clip_position;
    pbr_input.world_position = in.world_position;
    pbr_input.world_normal = prepare_world_normal(
        in.world_normal,
        false, // double_sided
    );
    
    pbr_input.is_orthographic = view.clip_from_view[3].w == 1.0;
    
    var color = apply_pbr_lighting(pbr_input);
    
    return alpha_discard(pbr_input.material, color);
}