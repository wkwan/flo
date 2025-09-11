#!/bin/bash

# Try to find a shader compiler
if command -v glslc &> /dev/null; then
    echo "Using glslc to compile shaders..."
    COMPILER="glslc"
    COMPILER_ARGS=""
elif command -v glslangValidator &> /dev/null; then
    echo "Using glslangValidator to compile shaders..."
    COMPILER="glslangValidator"
    COMPILER_ARGS="-V"
else
    echo "No shader compiler found. Please install glslc or glslangValidator."
    exit 1
fi

# Compile all vertex and fragment shaders
SHADER_DIR="shaders"
SUCCESS=true

# Find all .vert and .frag files and compile them
for shader in "$SHADER_DIR"/*.vert "$SHADER_DIR"/*.frag; do
    if [ -f "$shader" ]; then
        output="${shader}.spv"
        echo "Compiling $(basename "$shader") -> $(basename "$output")"
        
        if [ "$COMPILER" = "glslc" ]; then
            $COMPILER $COMPILER_ARGS "$shader" -o "$output"
        else
            $COMPILER $COMPILER_ARGS "$shader" -o "$output"
        fi
        
        if [ $? -ne 0 ]; then
            echo "Failed to compile $shader"
            SUCCESS=false
        fi
    fi
done

if [ "$SUCCESS" = true ]; then
    echo "All shaders compiled successfully!"
else
    echo "Some shaders failed to compile"
    exit 1
fi