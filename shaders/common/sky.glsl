// Common sky rendering functions

#ifndef SKY_GLSL
#define SKY_GLSL

vec3 getSkyColor(vec3 direction) {
    float gradientPos = clamp((direction.y + 0.5) / 1.5, 0.0, 1.0);
    
    vec3 bottomColor = vec3(0.8, 0.2, 0.1);
    vec3 horizonColor = vec3(1.0, 0.6, 0.3);
    vec3 midSkyColor = vec3(0.6, 0.7, 0.9);
    vec3 topColor = vec3(0.2, 0.4, 0.8);
    
    vec3 color;
    
    if (gradientPos < 0.3) {
        float t = gradientPos / 0.3;
        color = mix(bottomColor, horizonColor, smoothstep(0.0, 1.0, t));
    } else if (gradientPos < 0.7) {
        float t = (gradientPos - 0.3) / 0.4;
        color = mix(horizonColor, midSkyColor, smoothstep(0.0, 1.0, t));
    } else {
        float t = (gradientPos - 0.7) / 0.3;
        color = mix(midSkyColor, topColor, smoothstep(0.0, 1.0, t));
    }
    
    return color;
}

#endif // SKY_GLSL