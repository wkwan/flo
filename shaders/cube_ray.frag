#version 450

layout(location = 0) in vec3 fragNormal;
layout(location = 1) in vec3 fragPos;
layout(location = 2) in vec3 fragWorldPos;
layout(location = 3) in float objectType;  // 0.0 = cube, 1.0 = plane

layout(location = 0) out vec4 outColor;

// Performance quality settings (can be made uniforms for runtime adjustment)
const int QUALITY_LEVEL = 2; // 0=low, 1=medium, 2=high

// Performance tuning constants based on quality
const int MAX_SHADOW_STEPS = QUALITY_LEVEL == 0 ? 24 : QUALITY_LEVEL == 1 ? 32 : 48;
const int MAX_REFLECTION_STEPS = QUALITY_LEVEL == 0 ? 12 : QUALITY_LEVEL == 1 ? 20 : 32;
const int MAX_AO_STEPS = QUALITY_LEVEL == 0 ? 6 : QUALITY_LEVEL == 1 ? 8 : 12;
const float MAX_DIST = 20.0;
const float EPSILON = 0.001;
const float SHADOW_BIAS = 0.01;

// Material properties for the cube
const vec3 CUBE_ALBEDO = vec3(0.8, 0.8, 0.8);
const float CUBE_METALLIC = 0.2;
const float CUBE_ROUGHNESS = 0.3;
const float CUBE_AO = 0.5;

// Material properties for the plane
const vec3 PLANE_ALBEDO = vec3(0.6, 0.6, 0.6);  // Gray color
const float PLANE_METALLIC = 0.0;  // Non-metallic
const float PLANE_ROUGHNESS = 0.9;  // Very rough surface
const float PLANE_AO = 0.8;  // Higher AO for ground

// Multiple light sources for more realistic lighting
struct Light {
    vec3 position;
    vec3 color;
    float intensity;
    float radius;
};

const Light LIGHTS[4] = Light[4](
    Light(vec3(2.0, 2.0, 2.0), vec3(1.0, 0.95, 0.8), 1.2, 8.0),   // Main warm light
    Light(vec3(-2.0, 1.5, 1.0), vec3(0.8, 0.9, 1.0), 0.8, 6.0),    // Cool fill light
    Light(vec3(0.0, 3.0, 0.0), vec3(1.0, 1.0, 1.0), 0.6, 4.0),      // Top ambient light
    Light(vec3(0.0, 4.0, 0.0), vec3(1.0, 1.0, 1.0), 1.5, 6.0)      // Bright light above cube
);

const vec3 AMBIENT_COLOR = vec3(0.05, 0.08, 0.12);

// Improved SDF functions with better precision
float sdBox(vec3 p, vec3 b) {
    vec3 q = abs(p) - b;
    return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0);
}

float sdPlane(vec3 p, vec3 n, float h) {
    return dot(p, n) + h;
}

// Scene SDF - simplified scene with just cube and plane
float sceneSDF(vec3 p) {
    // Main cube
    float cube = sdBox(p, vec3(0.5));
    
    // Ground plane
    float ground = sdPlane(p, vec3(0.0, 1.0, 0.0), 1.0);
    
    // Just cube and plane - no decorative objects
    return min(cube, ground);
}

// Enhanced shadow calculation with smoother shadows
float calculateShadow(vec3 ro, vec3 rd, float maxDist) {
    float t = SHADOW_BIAS;
    float shadow = 1.0;
    float prevDist = 0.0;
    
    for (int i = 0; i < MAX_SHADOW_STEPS && t < maxDist; i++) {
        float h = sceneSDF(ro + rd * t);
        
        if (h < EPSILON) {
            return 0.0; // Early exit - hit something
        }
        
        // Darker shadow calculation
        float shadowFactor = 12.0 * h / t;  // Increased for darker shadows
        shadow = min(shadow, shadowFactor);
        
        // Adaptive step size based on distance
        float stepSize = max(h * 0.5, 0.05);
        t += stepSize;
        
        // Early exit if shadow is already very dark
        if (shadow < 0.02) break;  // Reduced for darker shadows
        
        prevDist = h;
    }
    
    // Apply darker shadow falloff
    return smoothstep(0.0, 0.8, shadow);  // Reduced for darker shadows
}

// Improved ambient occlusion with better sampling
float calculateAO(vec3 p, vec3 n) {
    float ao = 0.0;
    float weight = 1.0;
    
    for (int i = 0; i < MAX_AO_STEPS; i++) {
        float len = 0.01 + 0.2 * float(i) / float(MAX_AO_STEPS);
        vec3 samplePos = p + n * len;
        float dist = sceneSDF(samplePos);
        
        ao += (len - dist) * weight;
        weight *= 0.7;
    }
    
    return 1.0 - clamp(0.5 * ao, 0.0, 1.0);
}

// Enhanced reflection calculation with better quality
vec3 calculateReflection(vec3 ro, vec3 rd, vec3 normal, float roughness) {
    if (roughness > 0.9) return vec3(0.0); // Too rough for reflections
    
    vec3 reflectDir = reflect(rd, normal);
    float t = 0.01;
    vec3 reflection = vec3(0.0);
    
    // Adjust quality based on roughness
    int maxSteps = int(mix(float(MAX_REFLECTION_STEPS), 8.0, roughness));
    
    for (int i = 0; i < maxSteps; i++) {
        vec3 samplePos = ro + reflectDir * t;
        float dist = sceneSDF(samplePos);
        
        if (dist < EPSILON) {
            // Hit something - calculate reflection color
            vec3 hitNormal = normalize(vec3(
                sceneSDF(samplePos + vec3(EPSILON, 0.0, 0.0)) - sceneSDF(samplePos - vec3(EPSILON, 0.0, 0.0)),
                sceneSDF(samplePos + vec3(0.0, EPSILON, 0.0)) - sceneSDF(samplePos - vec3(0.0, EPSILON, 0.0)),
                sceneSDF(samplePos + vec3(0.0, 0.0, EPSILON)) - sceneSDF(samplePos - vec3(0.0, 0.0, EPSILON))
            ));
            
            // Enhanced environment mapping that better captures the blue sky
            vec3 envColor;
            if (hitNormal.y > 0.5) {
                // Top-facing surfaces get more of the light blue-white
                envColor = mix(vec3(0.6, 0.8, 1.0), vec3(0.4, 0.6, 0.9), 0.5);
            } else if (hitNormal.y < -0.5) {
                // Bottom-facing surfaces get more of the darker blue
                envColor = mix(vec3(0.2, 0.4, 0.8), vec3(0.1, 0.3, 0.7), 0.5);
            } else {
                // Side surfaces get a mix
                envColor = mix(vec3(0.3, 0.5, 0.85), vec3(0.2, 0.4, 0.8), 
                               smoothstep(-1.0, 1.0, hitNormal.y));
            }
            reflection = envColor;
            break;
        }
        
        t += dist * 0.5;
        if (t > 8.0) break; // Limit reflection distance
    }
    
    return reflection * (1.0 - roughness);
}

// PBR-inspired lighting calculation with multiple lights
vec3 calculateLighting(vec3 albedo, vec3 normal, vec3 viewDir, float metallic, float roughness, float ao, vec3 worldPos) {
    vec3 totalLighting = vec3(0.0);
    
    // Calculate lighting from each light source
    for (int i = 0; i < 4; i++) {
        Light light = LIGHTS[i];
        
        // Calculate light direction and distance
        vec3 lightDir = normalize(light.position - worldPos);
        float lightDist = distance(light.position, worldPos);
        
        // Attenuation based on distance
        float attenuation = 1.0 / (1.0 + lightDist * lightDist / (light.radius * light.radius));
        
        // Calculate shadows
        float shadow = calculateShadow(worldPos, lightDir, lightDist);
        
        // PBR lighting calculation
        vec3 halfDir = normalize(lightDir + viewDir);
        
        float NdotL = max(dot(normal, lightDir), 0.0);
        float NdotV = max(dot(normal, viewDir), 0.0);
        float NdotH = max(dot(normal, halfDir), 0.0);
        
        // Roughness-based lighting
        float roughness2 = roughness * roughness;
        
        // Fresnel calculation
        vec3 F0 = mix(vec3(0.04), albedo, metallic);
        vec3 F = F0 + (1.0 - F0) * pow(1.0 - NdotV, 5.0);
        
        // Diffuse and specular
        vec3 diffuse = albedo * (1.0 - metallic) * NdotL;
        vec3 specular = F * pow(NdotH, 1.0 / max(roughness2, 0.001)) * NdotL;
        
        // Combine lighting for this light
        vec3 lightContribution = (diffuse + specular) * light.color * light.intensity * attenuation * shadow;
        totalLighting += lightContribution;
    }
    
    // Ambient lighting
    vec3 ambient = albedo * AMBIENT_COLOR * ao;
    
    return ambient + totalLighting;
}

// Post-processing effects
vec3 applyPostProcessing(vec3 color) {
    // Tone mapping (Reinhard)
    color = color / (1.0 + color);
    
    // Contrast adjustment
    color = pow(color, vec3(0.9));
    
    // Slight color grading
    color.r = pow(color.r, 1.05);
    color.g = pow(color.g, 1.02);
    color.b = pow(color.b, 0.98);
    
    // Gamma correction
    color = pow(color, vec3(1.0 / 2.2));
    
    return color;
}

void main() {
    // Check if this fragment is part of the cube or plane
    if (length(fragNormal) > 0.1) {
        vec3 normal = normalize(fragNormal);
        vec3 viewDir = normalize(-fragWorldPos);
        
        // Determine material properties based on object type
        vec3 albedo;
        float metallic;
        float roughness;
        float ao;
        
        if (objectType < 0.5) {
            // Cube material
            albedo = CUBE_ALBEDO;
            metallic = CUBE_METALLIC;
            roughness = CUBE_ROUGHNESS;
            ao = CUBE_AO;
        } else {
            // Plane material
            albedo = PLANE_ALBEDO;
            metallic = PLANE_METALLIC;
            roughness = PLANE_ROUGHNESS;
            ao = PLANE_AO;
        }
        
        // Calculate ambient occlusion
        ao = calculateAO(fragWorldPos, normal);
        
        // Calculate reflections
        vec3 reflection = calculateReflection(fragWorldPos, viewDir, normal, roughness);
        
        // Calculate main lighting with multiple lights
        vec3 lighting = calculateLighting(
            albedo, 
            normal, 
            viewDir, 
            metallic, 
            roughness, 
            ao,
            fragWorldPos
        );
        
        // Add reflections
        lighting += reflection * metallic * 0.3;
        
        // Add subtle color variation based on world position (only for cube)
        if (objectType < 0.5) {
            vec3 colorVariation = vec3(0.03) * sin(fragWorldPos.x * 3.14159) * cos(fragWorldPos.z * 3.14159);
            lighting += colorVariation;
        }
        
        // Apply post-processing
        lighting = applyPostProcessing(lighting);
        
        outColor = vec4(lighting, 1.0);
    } else {
        // Background - bright blue gradient for better reflections
        float gradientPos = clamp((fragWorldPos.y + 1.0) / 2.0, 0.0, 1.0);
        vec3 bottomColor = vec3(0.2, 0.4, 0.8);   // Bright blue at bottom
        vec3 topColor = vec3(0.6, 0.8, 1.0);      // Light blue-white at top
        
        vec3 skyColor = mix(bottomColor, topColor, smoothstep(0.0, 1.0, gradientPos));
        
        // Add subtle noise to background
        float noise = sin(fragWorldPos.x * 10.0) * sin(fragWorldPos.z * 10.0) * 0.02;
        skyColor += vec3(noise);
        
        outColor = vec4(skyColor, 1.0);
    }
}
