// Common constants for all shaders

#ifndef CONSTANTS_GLSL
#define CONSTANTS_GLSL

// Lighting constants
const vec3 LIGHT_POSITION = vec3(20.0, 30.0, -20.0);
const vec3 LIGHT_COLOR = vec3(1.0, 0.9, 0.7);
const vec3 LIGHT_COLOR_WARM = vec3(1.0, 0.6, 0.3);
const float LIGHT_INTENSITY = 50000.0;

// Camera/view constants
const vec3 VIEW_POS = vec3(0.0, 6.0, 8.0);

// Mathematical constants
const float PI = 3.141592653589793;
const float TWO_PI = 6.283185307179586;
const float HALF_PI = 1.5707963267948966;

// Material constants
const float DEFAULT_SHININESS = 32.0;
const vec3 DEFAULT_AMBIENT = vec3(0.1);

#endif // CONSTANTS_GLSL