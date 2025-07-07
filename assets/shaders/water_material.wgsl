// reference https://www.shadertoy.com/view/MttfW8

#import bevy_pbr::forward_io::VertexOutput
#import bevy_pbr::mesh_view_bindings::view

// Lighting data passed from main.rs (currently as constants but available for use)
const LIGHT_POSITION: vec3<f32> = vec3<f32>(20.0, 30.0, -20.0);
const LIGHT_COLOR: vec3<f32> = vec3<f32>(1.0, 0.6, 0.3);
const LIGHT_INTENSITY: f32 = 50000.0;

// Ocean shader constants (from Seascape shader)
const NUM_STEPS: i32 = 8;
const PI: f32 = 3.141592;
const EPSILON: f32 = 1e-3;

// Sea parameters
const ITER_GEOMETRY: i32 = 3;
const ITER_FRAGMENT: i32 = 5;
const SEA_HEIGHT: f32 = 0.0;  // Water level relative to mesh
const SEA_CHOPPY: f32 = 4.0;
const SEA_SPEED: f32 = 0.8;
const SEA_FREQ: f32 = 0.16;
const SEA_BASE: vec3<f32> = vec3<f32>(0.05, 0.12, 0.18);  // Darker base for deeper water
const SEA_WATER_COLOR: vec3<f32> = vec3<f32>(0.3, 0.6, 0.7);  // More realistic water tint

// Octave matrix for wave generation
const OCTAVE_M: mat2x2<f32> = mat2x2<f32>(
    vec2<f32>(1.6, 1.2),
    vec2<f32>(-1.2, 1.6)
);

struct WaterMaterial {
    color: vec4<f32>,
    time: f32,
    camera_position: vec3<f32>,
    resolution: vec2<f32>,
    water_level: f32,
    grid_scale: f32,
};

@group(2) @binding(0) var<uniform> material: WaterMaterial;

// Lighting functions
fn diffuse(n: vec3<f32>, l: vec3<f32>, p: f32) -> f32 {
    // Calculate diffuse lighting with ambient term
    // dot(n, l) gives cosine of angle between normal and light (-1 to 1)
    // * 0.4 + 0.6 remaps to (0.2 to 1.0) to avoid fully dark areas
    // pow(..., p) sharpens the falloff curve
    return pow(dot(n, l) * 0.4 + 0.6, p);
}

fn specular(n: vec3<f32>, l: vec3<f32>, e: vec3<f32>, s: f32) -> f32 {
    // Calculate specular highlight (Blinn-Phong model)
    // nrm: normalization factor for energy conservation
    let nrm = (s + 8.0) / (PI * 8.0);
    // reflect(e, n): reflect eye vector around normal to get reflection direction
    // dot with light gives cosine of angle between reflection and light
    // pow(..., s): higher s = smaller, sharper highlight
    return pow(max(dot(reflect(e, n), l), 0.0), s) * nrm;
}

// Get sky color based on direction (same as sky gradient shader)
fn get_sky_color(direction: vec3<f32>) -> vec3<f32> {
    let gradient_pos = (direction.y + 1.0) * 0.5; // 0 = looking down, 1 = looking up
    
    // Sunrise colors (matching sky_gradient.wgsl)
    let horizon_color = vec3<f32>(1.0, 0.5, 0.2);      // Warm orange
    let lower_sky_color = vec3<f32>(0.9, 0.3, 0.1);    // Deep orange/red
    let upper_sky_color = vec3<f32>(0.4, 0.6, 0.9);    // Light blue
    let zenith_color = vec3<f32>(0.2, 0.4, 0.8);       // Deeper blue
    
    var color: vec3<f32>;
    
    if gradient_pos < 0.4 {
        let t = gradient_pos / 0.4;
        color = mix(lower_sky_color, horizon_color, smoothstep(0.0, 1.0, t));
    } else if gradient_pos < 0.6 {
        let t = (gradient_pos - 0.4) / 0.2;
        color = mix(horizon_color, upper_sky_color, smoothstep(0.0, 1.0, t));
    } else {
        let t = (gradient_pos - 0.6) / 0.4;
        color = mix(upper_sky_color, zenith_color, smoothstep(0.0, 1.0, t));
    }
    
    return color;
}

// Calculate water color with lighting and reflections
fn get_water_color(
    p: vec3<f32>,        // World position of water surface
    n: vec3<f32>,        // Surface normal
    l: vec3<f32>,        // Light direction (normalized)
    eye: vec3<f32>,      // View direction (from surface to camera, normalized)
    dist: vec3<f32>,     // Distance from camera to surface point
    water_level: f32     // Water level for depth calculation
) -> vec3<f32> {
    // Calculate Fresnel effect (more reflection at grazing angles)
    // 1.0 - dot(n, -eye): 0 when looking straight down, 1 at grazing angle
    var fresnel = clamp(1.0 - dot(n, -eye), 0.0, 1.0);
    // pow(fresnel, 3.0): makes transition sharper
    // * 0.65: reduces maximum reflectivity
    fresnel = pow(fresnel, 3.0) * 0.65;
    
    // Get reflected sky color
    // reflect(eye, n): calculate reflection direction
    let reflected = get_sky_color(reflect(eye, n));
    
    // Calculate refracted color (water body)
    // SEA_BASE: dark blue-green base color
    // diffuse lighting: adds directional lighting to water color
    // SEA_WATER_COLOR: water tint color
    // * 0.08: subtle tint intensity for more realistic water
    let refracted = SEA_BASE + diffuse(n, l, 40.0) * SEA_WATER_COLOR * 0.08;
    
    // Mix refraction and reflection based on Fresnel
    var color = mix(refracted, reflected, fresnel);
    
    // Add depth-based color (deeper water appears more colored)
    // Distance attenuation: fades effect with distance
    let atten = max(1.0 - dot(dist, dist) * 0.0001, 0.0);
    // Water gets darker with depth
    let depth_factor = smoothstep(-2.0, 0.0, p.y - water_level);
    color = mix(color * 0.3, color, depth_factor); // Darker at depth
    
    // Add subtle color variation based on depth
    color = color + SEA_WATER_COLOR * (1.0 - depth_factor) * 0.05 * atten;
    
    // Add specular highlights with sun color
    let spec = specular(n, l, eye, 80.0);
    color = color + LIGHT_COLOR * spec * 0.8;
    
    return color;
}

// Calculate normal using screen-space derivatives
fn get_normal_from_derivatives(world_normal: vec3<f32>, world_pos: vec3<f32>) -> vec3<f32> {
    // Use the interpolated vertex normal as base
    var normal = normalize(world_normal);
    
    // For additional detail, we could perturb the normal based on derivatives
    // But for now, we'll use the vertex normal which already contains height info
    
    // Ensure normal points up
    if normal.y < 0.0 {
        normal = -normal;
    }
    
    return normal;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Get world position from vertex
    let world_pos = in.world_position.xyz;
    
    // Calculate view direction (from surface to camera)
    let eye_dir = normalize(material.camera_position - world_pos);
    
    // Light direction (normalized)
    let light_dir = normalize(LIGHT_POSITION);
    
    // Get normal from vertex data (already contains height information)
    let normal = get_normal_from_derivatives(in.world_normal, world_pos);
    
    // DEBUG: Show normals as colors to check if they're varying
    // Uncomment this line to debug normals:
    // return vec4<f32>(normal * 0.5 + 0.5, 1.0);
    
    // Distance from camera to surface
    let dist = material.camera_position - world_pos;
    
    // Enhanced water color variation - more noticeable depth changes
    let height_factor = (world_pos.y - material.water_level + 3.0) / 6.0;  // Wider range
    let water_deep = vec3<f32>(0.05, 0.15, 0.4);     // Much darker deep blue
    let water_shallow = vec3<f32>(0.3, 0.8, 1.0);    // Bright cyan for shallow areas
    let simple_water = mix(water_deep, water_shallow, clamp(height_factor, 0.0, 1.0));
    
    // Enhanced lighting with more dramatic variation
    let ndotl = max(dot(normal, light_dir), 0.0);
    let lit_water = simple_water * (0.4 + 0.8 * ndotl);  // Stronger lighting contrast
    
    // Add depth-based darkening for more dramatic effect
    let depth_darkening = smoothstep(0.0, 0.5, 1.0 - height_factor);
    let darkened_water = mix(lit_water, lit_water * 0.3, depth_darkening);
    
    // Add simple Fresnel reflection
    let fresnel = pow(1.0 - max(dot(normal, eye_dir), 0.0), 2.0);
    let sky_color = get_sky_color(reflect(eye_dir, normal));
    let final_color = mix(darkened_water, sky_color, fresnel * 0.3);  // Reduced reflection for more color visibility
    
    // Return with original alpha from material
    return vec4<f32>(final_color, material.color.a);
}