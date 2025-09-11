// Common pattern generation functions

#ifndef PATTERNS_GLSL
#define PATTERNS_GLSL

vec3 calculateGridPattern(vec3 worldPos, float gridSize, float lineWidth, vec3 baseColor, vec3 gridColor) {
    // Calculate grid lines
    vec2 grid = abs(fract(worldPos.xz / gridSize - 0.5) - 0.5) / fwidth(worldPos.xz / gridSize);
    float line = min(grid.x, grid.y);
    
    // Mix based on whether we're on a grid line
    float gridStrength = 1.0 - min(line, 1.0);
    gridStrength = smoothstep(0.0, 0.1, gridStrength);
    
    return mix(baseColor, gridColor, gridStrength);
}

// Default grid pattern with standard colors
vec3 calculateDefaultGrid(vec3 worldPos) {
    float gridSize = 1.0; // 1 meter grid
    float lineWidth = 0.02; // 2cm wide lines
    vec3 baseColor = vec3(0.15, 0.15, 0.15); // dark gray floor
    vec3 gridColor = vec3(1.0, 0.0, 0.8); // pink/magenta color
    
    return calculateGridPattern(worldPos, gridSize, lineWidth, baseColor, gridColor);
}

#endif // PATTERNS_GLSL