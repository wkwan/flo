#version 450

#include "common/lighting.glsl"
#include "common/sky.glsl"
#include "common/constants.glsl"

layout(location = 0) in vec3 fragWorldPos;
layout(location = 1) in vec3 fragNormal;
layout(location = 2) in vec2 fragUV;
layout(location = 3) in vec3 fragCameraPos;
layout(location = 4) in float fragTime;
layout(location = 5) in float fragWaterLevel;

layout(location = 0) out vec4 outColor;

// Use LIGHT_COLOR_WARM from constants for water

// Sea parameters
const vec3 SEA_BASE = vec3(0.05, 0.12, 0.18);
const vec3 SEA_WATER_COLOR = vec3(0.3, 0.6, 0.7);

void main() {
    vec3 normal = normalize(fragNormal);
    
    // Ensure normal points up
    if (normal.y < 0.0) {
        normal = -normal;
    }
    
    // Calculate view direction (from surface to camera)
    vec3 eyeDir = normalize(fragCameraPos - fragWorldPos);
    
    // Light direction (normalized)
    vec3 lightDir = normalize(LIGHT_POSITION);
    
    // Calculate distance from camera
    vec3 dist = fragCameraPos - fragWorldPos;
    
    // Enhanced water color variation
    float heightFactor = (fragWorldPos.y - fragWaterLevel + 3.0) / 6.0;
    vec3 waterDeep = vec3(0.05, 0.15, 0.4);
    vec3 waterShallow = vec3(0.3, 0.8, 1.0);
    vec3 simpleWater = mix(waterDeep, waterShallow, clamp(heightFactor, 0.0, 1.0));
    
    // Enhanced lighting
    float ndotl = max(dot(normal, lightDir), 0.0);
    vec3 litWater = simpleWater * (0.4 + 0.8 * ndotl);
    
    // Add depth-based darkening
    float depthDarkening = smoothstep(0.0, 0.5, 1.0 - heightFactor);
    vec3 darkenedWater = mix(litWater, litWater * 0.3, depthDarkening);
    
    // Simple Fresnel reflection
    float fresnelEffect = fresnel(normal, eyeDir, 2.0);
    vec3 skyColor = getSkyColor(reflect(eyeDir, normal));
    vec3 finalColor = mix(darkenedWater, skyColor, fresnelEffect * 0.3);
    
    // Add specular highlights
    float spec = specularEnergy(normal, lightDir, eyeDir, 80.0);
    finalColor = finalColor + LIGHT_COLOR_WARM * spec * 0.8;
    
    outColor = vec4(finalColor, 0.8); // Semi-transparent water
}