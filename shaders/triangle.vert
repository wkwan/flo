#version 450

void main() {
    vec2 positions[3] = vec2[3](
        vec2(0.0, -0.5),
        vec2(0.5, 0.5),
        vec2(-0.5, 0.5)
    );
    
    gl_Position = vec4(positions[gl_VertexIndex], 0.0, 1.0);
}