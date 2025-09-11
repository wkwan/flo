#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_functions::get_world_from_local
#import bevy_pbr::view_transformations::position_world_to_clip

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = get_world_from_local(vertex.instance_index);
    out.world_position = world_from_local * vec4<f32>(vertex.position, 1.0);
    out.position = position_world_to_clip(out.world_position.xyz);
    out.color = vec4<f32>(vertex.color, 1.0);
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}