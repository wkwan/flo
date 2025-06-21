@group(0) @binding(0)
var texture_a: texture_storage_2d<rg32float, read_write>;

@group(0) @binding(1)  
var texture_b: texture_storage_2d<rg32float, read_write>;

@group(0) @binding(2)
var<uniform> simulation_params: SimulationParams;

struct SimulationParams {
    dampening: f32,
    input_x: f32,
    input_y: f32,
    input_size: f32,
    min_input_size: f32,
    got_input: f32,
    input_push: f32,
    resolution: vec2<f32>,
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<i32>(global_id.xy);
    let resolution = vec2<i32>(simulation_params.resolution);
    
    if (coords.x >= resolution.x || coords.y >= resolution.y) {
        return;
    }
    
    let uv = vec2<f32>(coords) / simulation_params.resolution;
    
    // Sample current and neighboring pixels (4-neighbor stencil)
    let current = textureLoad(texture_a, coords, 0);
    let left = textureLoad(texture_a, coords + vec2<i32>(-1, 0), 0);
    let right = textureLoad(texture_a, coords + vec2<i32>(1, 0), 0);
    let up = textureLoad(texture_a, coords + vec2<i32>(0, -1), 0);
    let down = textureLoad(texture_a, coords + vec2<i32>(0, 1), 0);
    
    // Wave equation: d = -(current.y - 0.5) * 2.0 + (neighbors - 2.0)
    var d = 0.0;
    let screenscale = simulation_params.resolution.x / 640.0;
    
    // Input handling
    if (simulation_params.got_input > 0.1) {
        let input_pos = vec2<f32>(simulation_params.input_x, simulation_params.input_y);
        let dist = length(input_pos - uv) * simulation_params.resolution.x;
        
        var val = 0.05 * smoothstep(simulation_params.input_size * screenscale, 
                                   simulation_params.min_input_size * screenscale, 
                                   dist);
        
        if (simulation_params.input_push > 0.1) {
            val *= -1.0;
        }
        d += val;
    }
    
    // Wave propagation (4-neighbor finite difference)
    d += -(current.y - 0.5) * 2.0 + (left.x + right.x + up.x + down.x - 2.0);
    d *= simulation_params.dampening;
    d = d * 0.5 + 0.5;
    
    // Store result: R = new height, G = previous height for velocity
    let result = vec4<f32>(d, current.x, 0.0, 0.0);
    textureStore(texture_b, coords, result);
}