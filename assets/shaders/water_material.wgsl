// Water material with wave displacement
#import bevy_pbr::mesh_functions::get_world_from_local;
#import bevy_render::view::View;

@group(0) @binding(0) var<uniform> view: View;
@group(2) @binding(0) var<uniform> material: WaterMaterialUniform;
@group(2) @binding(1) var wave_texture: texture_2d<f32>;
@group(2) @binding(2) var wave_sampler: sampler;

struct WaterMaterialUniform {
    wave_amplitude: f32,
    color: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
}

@vertex
fn vertex(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Sample wave height from texture
    let wave_data = textureSampleLevel(wave_texture, wave_sampler, vertex.uv, 0.0);
    let wave_height = (wave_data.r - 0.5) * material.wave_amplitude;
    
    // Apply vertex displacement
    var displaced_position = vertex.position;
    displaced_position.y += wave_height;
    
    let world_position = get_world_from_local(vertex.instance_index) * vec4<f32>(displaced_position, 1.0);
    out.clip_position = view.clip_from_world * world_position;
    
    // Calculate normal from wave gradients for proper lighting
    let uv_step = 1.0 / 512.0; // Texture resolution
    let wave_left = textureSampleLevel(wave_texture, wave_sampler, vertex.uv + vec2<f32>(-uv_step, 0.0), 0.0).r;
    let wave_right = textureSampleLevel(wave_texture, wave_sampler, vertex.uv + vec2<f32>(uv_step, 0.0), 0.0).r;
    let wave_up = textureSampleLevel(wave_texture, wave_sampler, vertex.uv + vec2<f32>(0.0, -uv_step), 0.0).r;
    let wave_down = textureSampleLevel(wave_texture, wave_sampler, vertex.uv + vec2<f32>(0.0, uv_step), 0.0).r;
    
    let gradient_x = (wave_right - wave_left) * material.wave_amplitude * 0.5;
    let gradient_z = (wave_down - wave_up) * material.wave_amplitude * 0.5;
    
    let surface_normal = normalize(vec3<f32>(-gradient_x, 1.0, -gradient_z));
    out.world_normal = surface_normal;
    
    out.uv = vertex.uv;
    return out;
}

@fragment  
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple water color with normal-based shading
    let light_dir = normalize(vec3<f32>(0.0, 1.0, 0.0));
    let ndotl = max(dot(in.world_normal, light_dir), 0.0);
    
    let base_color = material.color.rgb;
    let lit_color = base_color * (0.3 + 0.7 * ndotl);
    
    return vec4<f32>(lit_color, material.color.a);
}