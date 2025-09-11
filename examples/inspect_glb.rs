use gltf::Gltf;
use std::env;

fn main() {
    // Get the GLB file path from command line arguments
    let args: Vec<String> = env::args().collect();
    let glb_path = if args.len() > 1 {
        &args[1]
    } else {
        "assets/red_grapes_wjbgdiz_low.glb"
    };
    
    println!("Inspecting GLB file: {}", glb_path);
    println!("{}", "=".repeat(50));
    
    // Load the GLB file
    let gltf = match Gltf::open(glb_path) {
        Ok(gltf) => gltf,
        Err(e) => {
            eprintln!("Error loading GLB file: {}", e);
            return;
        }
    };
    
    let document = gltf.document;
    
    // Check for images/textures
    let images: Vec<_> = document.images().collect();
    println!("Images/Textures found: {}", images.len());
    
    if !images.is_empty() {
        for (i, image) in images.iter().enumerate() {
            println!("  Image {}: {:?}", i, image.name());
            
            match image.source() {
                gltf::image::Source::View { view, mime_type } => {
                    println!("    Format: {} (embedded in GLB)", mime_type);
                    println!("    Size: {} bytes", view.length());
                }
                gltf::image::Source::Uri { uri, mime_type } => {
                    println!("    Format: {:?}", mime_type);
                    println!("    URI: {}", uri);
                }
            }
        }
    } else {
        println!("  No images/textures found in this GLB file.");
    }
    
    println!();
    
    // Check materials
    let materials: Vec<_> = document.materials().collect();
    println!("Materials found: {}", materials.len());
    
    if !materials.is_empty() {
        for (i, material) in materials.iter().enumerate() {
            println!("  Material {}: {:?}", i, material.name().unwrap_or("Unnamed"));
            
            let pbr = material.pbr_metallic_roughness();
            
            // Base color
            if let Some(base_color_texture) = pbr.base_color_texture() {
                println!("    - Base color texture: Image index {}", base_color_texture.texture().source().index());
            } else {
                let base_color = pbr.base_color_factor();
                println!("    - Base color: [{:.3}, {:.3}, {:.3}, {:.3}]", 
                    base_color[0], base_color[1], base_color[2], base_color[3]);
            }
            
            // Metallic/roughness
            if let Some(mr_texture) = pbr.metallic_roughness_texture() {
                println!("    - Metallic/Roughness texture: Image index {}", mr_texture.texture().source().index());
            } else {
                println!("    - Metallic factor: {:.3}", pbr.metallic_factor());
                println!("    - Roughness factor: {:.3}", pbr.roughness_factor());
            }
            
            // Normal map
            if let Some(normal_texture) = material.normal_texture() {
                println!("    - Normal texture: Image index {}", normal_texture.texture().source().index());
            }
            
            // Occlusion map
            if let Some(occlusion_texture) = material.occlusion_texture() {
                println!("    - Occlusion texture: Image index {}", occlusion_texture.texture().source().index());
            }
            
            // Emissive
            if let Some(emissive_texture) = material.emissive_texture() {
                println!("    - Emissive texture: Image index {}", emissive_texture.texture().source().index());
            } else {
                let emissive = material.emissive_factor();
                if emissive != [0.0, 0.0, 0.0] {
                    println!("    - Emissive factor: [{:.3}, {:.3}, {:.3}]", 
                        emissive[0], emissive[1], emissive[2]);
                }
            }
        }
    } else {
        println!("  No materials found in this GLB file.");
    }
    
    println!();
    
    // Additional file info
    println!("Additional Information:");
    println!("  Meshes: {}", document.meshes().count());
    println!("  Nodes: {}", document.nodes().count());
    println!("  Scenes: {}", document.scenes().count());
    println!("  Animations: {}", document.animations().count());
    
    // Check file size
    if let Ok(metadata) = std::fs::metadata(glb_path) {
        println!("  File size: {} bytes ({:.1} KB)", metadata.len(), metadata.len() as f64 / 1024.0);
    }
}