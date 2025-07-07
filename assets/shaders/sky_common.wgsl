// Shared sky gradient function used by both sky and water shaders

// Get sky color based on direction - enhanced for more visible gradient
fn get_sky_color(direction: vec3<f32>) -> vec3<f32> {
    // Map Y direction to gradient position with more dramatic range
    // -0.5 to 1.0 gives us a wider visible range from current camera angle
    let gradient_pos = clamp((direction.y + 0.5) / 1.5, 0.0, 1.0);
    
    // Enhanced colors for more dramatic gradient
    let bottom_color = vec3<f32>(0.8, 0.2, 0.1);       // Deep red-orange
    let horizon_color = vec3<f32>(1.0, 0.6, 0.3);      // Bright orange
    let mid_sky_color = vec3<f32>(0.6, 0.7, 0.9);      // Light blue
    let top_color = vec3<f32>(0.2, 0.4, 0.8);          // Deep blue
    
    var color: vec3<f32>;
    
    if gradient_pos < 0.3 {
        // Bottom to horizon
        let t = gradient_pos / 0.3;
        color = mix(bottom_color, horizon_color, smoothstep(0.0, 1.0, t));
    } else if gradient_pos < 0.7 {
        // Horizon to mid sky
        let t = (gradient_pos - 0.3) / 0.4;
        color = mix(horizon_color, mid_sky_color, smoothstep(0.0, 1.0, t));
    } else {
        // Mid sky to top
        let t = (gradient_pos - 0.7) / 0.3;
        color = mix(mid_sky_color, top_color, smoothstep(0.0, 1.0, t));
    }
    
    return color;
}