#version 450

#include "common/lighting.glsl"
#include "common/constants.glsl"
#include "common/materials.glsl"

layout(location = 0) in vec3 fragWorldPos;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragUV;

layout(location = 0) out vec4 outColor;

// Texture samplers
layout(binding = 0) uniform sampler2D baseColorSampler;
layout(binding = 1) uniform sampler2D normalSampler;
layout(binding = 2) uniform sampler2D roughnessSampler;
layout(binding = 3) uniform sampler2D aoSampler;


void main() {
    // Sample textures with tiling
    // Use different tiling for better appearance with taller walls
    vec2 tiledUV = fragUV * vec2(3.0, 3.0); // Tile the texture 3x3
    
    vec3 baseColor = texture(baseColorSampler, tiledUV).rgb;
    float roughness = texture(roughnessSampler, tiledUV).r;
    float ao = texture(aoSampler, tiledUV).r;
    
    // Get normal from normal map
    vec3 normal = getNormalFromMap(normalSampler, tiledUV, fragWorldPos, fragNormal);
    
    // Calculate lighting
    vec3 lightDir = normalize(LIGHT_POSITION - fragWorldPos);
    vec3 viewDir = normalize(VIEW_POS - fragWorldPos);
    vec3 halfwayDir = normalize(lightDir + viewDir);
    
    // Diffuse
    float NdotL = calculateDiffuse(normal, lightDir);
    vec3 diffuse = baseColor * LIGHT_COLOR * NdotL;
    
    // Specular (Blinn-Phong with roughness)
    float spec = calculateBlinnPhongSpecular(normal, lightDir, viewDir, mix(128.0, 8.0, roughness));
    vec3 specular = LIGHT_COLOR * spec * (1.0 - roughness);
    
    // Ambient with AO
    vec3 ambient = baseColor * 0.3 * ao;
    
    vec3 finalColor = ambient + diffuse * 0.7 + specular * 0.2;
    
    // Add depth-based darkening
    float depthDarkening = smoothstep(-1.0, 3.0, fragWorldPos.y);
    finalColor *= (0.6 + 0.4 * depthDarkening);
    
    outColor = vec4(finalColor, 1.0);
}