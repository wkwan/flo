use bevy::prelude::*;

#[derive(Default)]
pub struct FpsLogger {
    frame_count: u32,
    fps_frame_count: u32,
    last_fps_time: f32,
}

impl FpsLogger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, time: &Time) {
        self.frame_count += 1;
        self.fps_frame_count += 1;

        if self.frame_count == 1 {
            println!("Starting render loop...");
        }

        let current_time = time.elapsed_secs();
        let time_since_last_fps = current_time - self.last_fps_time;

        if time_since_last_fps >= 1.0 {
            let fps = self.fps_frame_count as f32 / time_since_last_fps;
            let frame_time_ms = time_since_last_fps * 1000.0 / self.fps_frame_count as f32;

            println!("FPS: {:.1} | Frame Time: {:.2}ms", fps, frame_time_ms);

            self.last_fps_time = current_time;
            self.fps_frame_count = 0;
        }
    }
}