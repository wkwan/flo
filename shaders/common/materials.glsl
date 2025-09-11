// Common material and shading functions

#ifndef MATERIALS_GLSL
#define MATERIALS_GLSL

// Calculate emissive white material
vec3 calculateEmissiveWhite(vec3 normal) {
    vec3 baseColor = vec3(1.0, 1.0, 1.0);
    
    // Simple lighting
    vec3 lightDir = normalize(vec3(0.5, 1.0, 0.3));
    float diff = max(dot(normalize(normal), lightDir), 0.0);
    
    // High ambient and emissive to make it bright white
    vec3 ambient = vec3(0.5);
    vec3 emissive = vec3(0.3);
    
    return baseColor * (ambient + diff * 0.5) + emissive;
}

// Get normal from normal map using TBN matrix
vec3 getNormalFromMap(sampler2D normalMap, vec2 uv, vec3 worldPos, vec3 normal) {
    vec3 tangentNormal = texture(normalMap, uv).xyz * 2.0 - 1.0;
    
    vec3 Q1 = dFdx(worldPos);
    vec3 Q2 = dFdy(worldPos);
    vec2 st1 = dFdx(uv);
    vec2 st2 = dFdy(uv);
    
    vec3 N = normalize(normal);
    vec3 T = normalize(Q1 * st2.t - Q2 * st1.t);
    vec3 B = -normalize(cross(N, T));
    mat3 TBN = mat3(T, B, N);
    
    return normalize(TBN * tangentNormal);
}

// Simple directional lighting calculation
vec3 calculateSimpleDirectionalLight(vec3 normal, vec3 lightDir, vec3 baseColor) {
    float diff = max(dot(normalize(normal), normalize(lightDir)), 0.0);
    return baseColor * diff;
}

// Standard simple lighting with default light direction
vec3 calculateSimpleLighting(vec3 normal, vec3 baseColor) {
    vec3 lightDir = normalize(vec3(0.5, 1.0, 0.3));
    return calculateSimpleDirectionalLight(normal, lightDir, baseColor);
}

#endif // MATERIALS_GLSL