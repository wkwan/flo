use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapperHolder};
use bevy::math::{Mat4, Vec3};
use bevy::input::mouse::{MouseButton, MouseMotion, MouseWheel};

use vulkan_bevy_renderer::{
    setup_bevy_app, 
    vulkan_renderer_unified::VulkanRenderer,
    fps_logger::FpsLogger,
};

fn main() {
    let mut app = setup_bevy_app();
    
    app.init_resource::<EguiInputState>()
        .add_systems(PostStartup, setup_vulkan_renderer)
        .add_systems(
            Update,
            (
                collect_egui_input,
                render_frame,
            ).chain().run_if(resource_exists::<VulkanContext>),
        )
        .run();
}

#[derive(Resource)]
struct VulkanContext {
    renderer: VulkanRenderer,
    fps_logger: FpsLogger,
    demo_text: String,
    slider_value: f32,
    checkbox_value: bool,
}

#[derive(Resource, Default)]
struct EguiInputState {
    events: Vec<egui::Event>,
    cursor_pos: Option<egui::Pos2>,
}

fn collect_egui_input(
    mut egui_input: ResMut<EguiInputState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    _mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // Clear previous frame's events
    egui_input.events.clear();
    
    let window = windows.single().expect("Failed to get primary window");
    
    // Update cursor position
    if let Some(cursor_pos) = window.cursor_position() {
        let pos = egui::pos2(cursor_pos.x, cursor_pos.y);
        egui_input.cursor_pos = Some(pos);
        egui_input.events.push(egui::Event::PointerMoved(pos));
    }
    
    // Handle mouse button events
    if let Some(cursor_pos) = egui_input.cursor_pos {
        if mouse_button_input.just_pressed(MouseButton::Left) {
            egui_input.events.push(egui::Event::PointerButton {
                pos: cursor_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            });
        }
        
        if mouse_button_input.just_released(MouseButton::Left) {
            egui_input.events.push(egui::Event::PointerButton {
                pos: cursor_pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            });
        }
        
        if mouse_button_input.just_pressed(MouseButton::Right) {
            egui_input.events.push(egui::Event::PointerButton {
                pos: cursor_pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            });
        }
        
        if mouse_button_input.just_released(MouseButton::Right) {
            egui_input.events.push(egui::Event::PointerButton {
                pos: cursor_pos,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            });
        }
    }
    
    // Handle mouse wheel events
    for event in mouse_wheel_events.read() {
        egui_input.events.push(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(event.x, event.y),
            modifiers: egui::Modifiers::default(),
        });
    }
    
    // Handle keyboard events
    for key in keyboard_input.get_just_pressed() {
        if let Some(egui_key) = bevy_key_to_egui(*key) {
            egui_input.events.push(egui::Event::Key {
                key: egui_key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            });
        }
    }
    
    for key in keyboard_input.get_just_released() {
        if let Some(egui_key) = bevy_key_to_egui(*key) {
            egui_input.events.push(egui::Event::Key {
                key: egui_key,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            });
        }
    }
    
    // For text input, we rely on keyboard events combined with modifiers
    // This is a simplified approach - a full implementation would need proper text input handling
}

fn bevy_key_to_egui(key: KeyCode) -> Option<egui::Key> {
    match key {
        KeyCode::ArrowLeft => Some(egui::Key::ArrowLeft),
        KeyCode::ArrowRight => Some(egui::Key::ArrowRight),
        KeyCode::ArrowUp => Some(egui::Key::ArrowUp),
        KeyCode::ArrowDown => Some(egui::Key::ArrowDown),
        KeyCode::Escape => Some(egui::Key::Escape),
        KeyCode::Tab => Some(egui::Key::Tab),
        KeyCode::Backspace => Some(egui::Key::Backspace),
        KeyCode::Enter => Some(egui::Key::Enter),
        KeyCode::Space => Some(egui::Key::Space),
        KeyCode::Insert => Some(egui::Key::Insert),
        KeyCode::Delete => Some(egui::Key::Delete),
        KeyCode::Home => Some(egui::Key::Home),
        KeyCode::End => Some(egui::Key::End),
        KeyCode::PageUp => Some(egui::Key::PageUp),
        KeyCode::PageDown => Some(egui::Key::PageDown),
        KeyCode::KeyA => Some(egui::Key::A),
        KeyCode::KeyC => Some(egui::Key::C),
        KeyCode::KeyV => Some(egui::Key::V),
        KeyCode::KeyX => Some(egui::Key::X),
        KeyCode::KeyZ => Some(egui::Key::Z),
        _ => None,
    }
}

fn setup_vulkan_renderer(
    mut commands: Commands,
    windows: Query<(Entity, &RawHandleWrapperHolder, &Window), With<PrimaryWindow>>,
) {
    let (_entity, handle_wrapper, window) = windows.single().expect("Failed to get primary window");
    
    println!("Window size: {:?}x{:?}", window.width(), window.height());
    println!("Initializing egui example...");
    
    // Create a simple triangle renderer
    let mut renderer = VulkanRenderer::new_simple(
        handle_wrapper,
        "shaders/triangle.vert.spv",
        "shaders/triangle.frag.spv",
        3,
    ).expect("Failed to create Vulkan renderer");
    
    // Initialize egui integration
    let render_pass = renderer.get_render_pass();
    renderer.initialize_egui(render_pass)
        .expect("Failed to initialize egui");
    
    commands.insert_resource(VulkanContext { 
        renderer,
        fps_logger: FpsLogger::new(),
        demo_text: String::from("Hello from egui!"),
        slider_value: 0.5,
        checkbox_value: true,
    });
}

fn render_frame(
    mut vulkan: ResMut<VulkanContext>,
    time: Res<Time>,
    windows: Query<&Window, With<PrimaryWindow>>,
    egui_input: Res<EguiInputState>,
) {
    vulkan.fps_logger.update(&time);
    
    let window = windows.single().expect("Failed to get primary window");
    let width = window.width();
    let height = window.height();
    
    // Extract values we need for UI before borrowing egui context
    let mut demo_text = vulkan.demo_text.clone();
    let mut slider_value = vulkan.slider_value;
    let mut checkbox_value = vulkan.checkbox_value;
    let frame_time_ms = time.delta_secs() * 1000.0;
    
    // Get the egui context and run UI code
    let egui_output = if let Some(ctx) = vulkan.renderer.get_egui_context() {
        // Create raw input for egui with collected events
        let mut raw_input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, height),
            )),
            time: Some(time.elapsed_secs_f64()),
            predicted_dt: time.delta_secs(),
            ..Default::default()
        };
        
        // Add collected input events
        raw_input.events = egui_input.events.clone();
        
        // Begin egui frame
        ctx.begin_pass(raw_input);
        
        // Draw UI
        egui::Window::new("Egui Demo")
            .default_pos(egui::pos2(50.0, 50.0))
            .show(ctx, |ui| {
                ui.heading("Vulkan + Egui Integration");
                
                ui.separator();
                
                ui.label("This is a demo of egui running on Vulkan!");
                
                ui.horizontal(|ui| {
                    ui.label("Text input:");
                    ui.text_edit_singleline(&mut demo_text);
                });
                
                ui.add(egui::Slider::new(&mut slider_value, 0.0..=1.0)
                    .text("Slider"));
                
                ui.checkbox(&mut checkbox_value, "Checkbox");
                
                if ui.button("Click me!").clicked() {
                    println!("Button clicked!");
                    demo_text = String::from("Button was clicked!");
                }
                
                ui.separator();
                
                // Show FPS info
                let fps_text = format!("Frame time: {:.2}ms", frame_time_ms);
                ui.label(fps_text);
            });
        
        // Show another window with different content
        egui::Window::new("Settings")
            .default_pos(egui::pos2(300.0, 50.0))
            .show(ctx, |ui| {
                ui.heading("Settings Panel");
                
                ui.collapsing("Advanced", |ui| {
                    ui.label("Advanced settings go here");
                    ui.label(format!("Slider value: {:.2}", slider_value));
                    ui.label(format!("Checkbox: {}", checkbox_value));
                });
                
                ui.separator();
                
                if ui.button("Reset").clicked() {
                    slider_value = 0.5;
                    checkbox_value = true;
                    demo_text = String::from("Hello from egui!");
                }
            });
        
        // End egui frame
        let output = ctx.end_pass();
        
        // Save modified values back to resource
        vulkan.demo_text = demo_text;
        vulkan.slider_value = slider_value;
        vulkan.checkbox_value = checkbox_value;
        
        Some(output)
    } else {
        None
    };
    
    // Setup view and projection matrices
    let aspect_ratio = width / height;
    let view = Mat4::look_at_rh(
        Vec3::new(0.0, 0.0, 3.0),
        Vec3::ZERO,
        Vec3::Y,
    );
    let proj = Mat4::perspective_rh(
        45.0_f32.to_radians(),
        aspect_ratio,
        0.1,
        100.0,
    );
    
    // Render frame with egui output
    vulkan.renderer.render_frame_with_egui(view, proj, egui_output);
}