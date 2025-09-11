use bevy::prelude::*;
use vulkan_bevy_renderer::fps_logger::FpsLogger;
use bevy::asset::{AssetPlugin, RenderAssetUsages};
use bevy::render::RenderPlugin;
use bevy::pbr::PbrPlugin;
use bevy::core_pipeline::CorePipelinePlugin;
use bevy::winit::{WinitPlugin, WakeUp};
use bevy::render::texture::ImagePlugin;
use bevy::window::{WindowPlugin, Window, PresentMode};
use bevy::a11y::AccessibilityPlugin;
use bevy::scene::ScenePlugin;
use bevy::transform::TransformPlugin;
use bevy::input::InputPlugin;
use bevy_egui::{egui, EguiContexts, EguiPlugin, EguiPrimaryContextPass};

fn main() {
    App::new()
        .add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::Immediate, // Disable vsync
                    ..default()
                }),
                ..default()
            },
            AccessibilityPlugin,
            InputPlugin::default(),
            WinitPlugin::<WakeUp>::default(),
            TransformPlugin,
            RenderPlugin::default(),
            ImagePlugin::default(),
            CorePipelinePlugin::default(),
            ScenePlugin,
            PbrPlugin::default(),
            EguiPlugin::default(),
        ))
        .init_resource::<UiState>()
        .add_systems(Startup, setup)
        .add_systems(Update, log_fps)
        .add_systems(EguiPrimaryContextPass, ui_system)
        .run();
}

#[derive(Resource)]
struct UiState {
    demo_text: String,
    slider_value: f32,
    checkbox_value: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            demo_text: String::from("Hello from egui!"),
            slider_value: 0.5,
            checkbox_value: true,
        }
    }
}

fn setup(
    mut commands: Commands,
) {
    println!("Setting up egui scene with Bevy...");
    
    // Add a camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    println!("Egui scene setup complete");
}

fn ui_system(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    time: Res<Time>,
) {
    // Main window (similar to the Vulkan egui example)
    egui::Window::new("Egui Demo")
        .default_pos(egui::pos2(50.0, 50.0))
        .show(contexts.ctx_mut().unwrap(), |ui| {
            ui.heading("Vulkan + Egui Integration (Bevy)");
            
            ui.separator();
            
            ui.label("This is a demo of egui running with Bevy!");
            
            ui.horizontal(|ui| {
                ui.label("Text input:");
                ui.text_edit_singleline(&mut ui_state.demo_text);
            });
            
            ui.add(egui::Slider::new(&mut ui_state.slider_value, 0.0..=1.0)
                .text("Slider"));
            
            ui.checkbox(&mut ui_state.checkbox_value, "Checkbox");
            
            if ui.button("Click me!").clicked() {
                println!("Button clicked!");
                ui_state.demo_text = String::from("Button was clicked!");
            }
            
            ui.separator();
            
            // Show FPS info
            let frame_time_ms = time.delta_secs() * 1000.0;
            let fps_text = format!("Frame time: {:.2}ms", frame_time_ms);
            ui.label(fps_text);
        });
    
    // Settings window (second window like in the Vulkan example)
    egui::Window::new("Settings")
        .default_pos(egui::pos2(300.0, 50.0))
        .show(contexts.ctx_mut().unwrap(), |ui| {
            ui.heading("Settings Panel");
            
            ui.collapsing("Advanced", |ui| {
                ui.label("Advanced settings go here");
                ui.label(format!("Slider value: {:.2}", ui_state.slider_value));
                ui.label(format!("Checkbox: {}", ui_state.checkbox_value));
            });
            
            ui.separator();
            
            if ui.button("Reset").clicked() {
                ui_state.slider_value = 0.5;
                ui_state.checkbox_value = true;
                ui_state.demo_text = String::from("Hello from egui!");
            }
        });
}

fn log_fps(
    mut fps_logger: Local<FpsLogger>,
    time: Res<Time>,
) {
    fps_logger.update(&time);
}