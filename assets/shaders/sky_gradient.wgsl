#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view
#import "shaders/sky_common.wgsl"::get_sky_color

struct SkyMaterial {
    camera_position: vec3<f32>,
    _padding: f32,
};

@group(2) @binding(0) var<uniform> material: SkyMaterial;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Get world position from vertex
    let world_pos = in.world_position.xyz;
    
    // Calculate view direction (from camera to this pixel)
    let view_dir = normalize(world_pos - material.camera_position);
    
    // Get sky color based on view direction
    let sky_color = get_sky_color(view_dir);
    
    return vec4<f32>(sky_color, 1.0);
}