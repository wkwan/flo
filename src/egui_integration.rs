use ash::vk;
use bevy::prelude::*;
use egui_ash_renderer::{Renderer, Options};

pub struct EguiIntegration {
    pub renderer: Renderer,
    pub context: egui::Context,
    pub queue: vk::Queue,
    pub command_pool: vk::CommandPool,
}

impl EguiIntegration {
    pub fn new(
        instance: &ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: ash::Device,
        render_pass: vk::RenderPass,
        queue: vk::Queue,
        command_pool: vk::CommandPool,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let renderer = Renderer::with_default_allocator(
            instance,
            physical_device,
            device,
            render_pass,
            Options::default(),
        )?;

        let context = egui::Context::default();

        Ok(Self {
            renderer,
            context,
            queue,
            command_pool,
        })
    }

    pub fn begin_frame(&mut self, raw_input: egui::RawInput, pixels_per_point: f32) {
        // Set pixels_per_point before beginning the frame
        // This ensures proper scaling for both rendering and interaction
        self.context.set_pixels_per_point(pixels_per_point);
        self.context.begin_pass(raw_input);
    }

    pub fn end_frame(&mut self) -> egui::FullOutput {
        self.context.end_pass()
    }

    pub fn paint(
        &mut self,
        command_buffer: vk::CommandBuffer,
        extent: vk::Extent2D,
        full_output: egui::FullOutput,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let clipped_primitives = self.context.tessellate(
            full_output.shapes,
            full_output.pixels_per_point,
        );
        
        // Set new and updated textures
        if !full_output.textures_delta.set.is_empty() {
            self.renderer.set_textures(
                self.queue,
                self.command_pool,
                full_output.textures_delta.set.as_slice(),
            )?;
        }
        
        self.renderer.cmd_draw(
            command_buffer,
            extent,
            full_output.pixels_per_point,
            &clipped_primitives,
        )?;
        
        // Free removed textures
        if !full_output.textures_delta.free.is_empty() {
            self.renderer.free_textures(&full_output.textures_delta.free)?;
        }

        Ok(())
    }

    pub fn update_swapchain(&mut self, _width: u32, _height: u32) {
        // egui-ash-renderer handles this internally
    }

    pub fn cleanup(&mut self) {
        // Renderer cleanup is handled in Drop trait
    }
}

// Bevy resource wrapper for egui context
// This holds the raw input and a reference to the context in the renderer
#[derive(Resource)]
pub struct EguiContext {
    pub raw_input: egui::RawInput,
    pub has_context: bool, // Track if renderer has been initialized with egui
    pub scale_factor: f32, // Store the current display scale factor
}

impl Default for EguiContext {
    fn default() -> Self {
        Self {
            raw_input: egui::RawInput::default(),
            has_context: false,
            scale_factor: 1.0,
        }
    }
}

// System to handle egui input from Bevy
pub fn update_egui_input(
    mut egui_ctx: ResMut<EguiContext>,
    windows: Query<&Window>,
    time: Res<Time>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    if let Ok(window) = windows.single() {
        // Get the scale factor from the window and store it in the resource
        let scale_factor = window.scale_factor();
        egui_ctx.scale_factor = scale_factor;
        
        let raw_input = &mut egui_ctx.raw_input;
        
        
        // Update screen rect using physical dimensions to fill the entire window
        raw_input.screen_rect = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(window.physical_width() as f32, window.physical_height() as f32),
        ));
        
        // Update time
        raw_input.time = Some(time.elapsed_secs_f64());
        
        // Track cursor position for button events
        let mut last_cursor_pos = egui::pos2(0.0, 0.0);
        
        // Update mouse position
        // Scale to physical coordinates to match screen rect
        if let Some(cursor_pos) = window.cursor_position() {
            last_cursor_pos = egui::pos2(cursor_pos.x * scale_factor, cursor_pos.y * scale_factor);
            raw_input.events.push(egui::Event::PointerMoved(last_cursor_pos));
        }
        
        // Update mouse buttons using the last known cursor position
        if mouse_button_input.just_pressed(MouseButton::Left) {
            raw_input.events.push(egui::Event::PointerButton {
                pos: last_cursor_pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            });
        }
        
        if mouse_button_input.just_released(MouseButton::Left) {
            raw_input.events.push(egui::Event::PointerButton {
                pos: last_cursor_pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::default(),
            });
        }
        
        // Add basic keyboard input handling
        for key in keyboard_input.get_just_pressed() {
            if let Some(egui_key) = bevy_key_to_egui(*key) {
                raw_input.events.push(egui::Event::Key {
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
                raw_input.events.push(egui::Event::Key {
                    key: egui_key,
                    physical_key: None,
                    pressed: false,
                    repeat: false,
                    modifiers: egui::Modifiers::default(),
                });
            }
        }
    }
}

fn bevy_key_to_egui(key: KeyCode) -> Option<egui::Key> {
    match key {
        KeyCode::Space => Some(egui::Key::Space),
        KeyCode::Enter => Some(egui::Key::Enter),
        KeyCode::Tab => Some(egui::Key::Tab),
        KeyCode::Backspace => Some(egui::Key::Backspace),
        KeyCode::Delete => Some(egui::Key::Delete),
        KeyCode::ArrowLeft => Some(egui::Key::ArrowLeft),
        KeyCode::ArrowRight => Some(egui::Key::ArrowRight),
        KeyCode::ArrowUp => Some(egui::Key::ArrowUp),
        KeyCode::ArrowDown => Some(egui::Key::ArrowDown),
        KeyCode::Home => Some(egui::Key::Home),
        KeyCode::End => Some(egui::Key::End),
        KeyCode::PageUp => Some(egui::Key::PageUp),
        KeyCode::PageDown => Some(egui::Key::PageDown),
        KeyCode::Escape => Some(egui::Key::Escape),
        _ => None,
    }
}

// Helper to get egui context for UI code - requires access to the renderer
// This is a placeholder - in actual use, you need to get the context from the renderer
pub fn get_egui_context(_egui_ctx: &mut EguiContext) -> Option<&egui::Context> {
    // The actual context is in the renderer - this needs to be refactored
    None
}