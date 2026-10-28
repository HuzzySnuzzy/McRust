use crate::camera::Camera;
use crate::renderer::Renderer;
use anyhow::Result;
use glam::Vec3;
use mcrust_world::mesh::mesh_chunk;
use mcrust_world::terrain::TerrainGenerator;
use mcrust_world::{ChunkPos, World};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{CursorGrabMode, Window, WindowId};

/// Chunks generated in each direction around the spawn chunk.
const VIEW_DISTANCE: i32 = 8;
const SEED: u32 = 2026;

struct Game {
    window: Arc<Window>,
    renderer: Renderer,
    camera: Camera,
    keys: HashSet<KeyCode>,
    mouse_grabbed: bool,
    last_frame: Instant,
    fps_timer: Instant,
    frames: u32,
}

#[derive(Default)]
struct App {
    game: Option<Game>,
}

pub fn run() -> Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn build_world() -> World {
    let started = Instant::now();
    let generator = TerrainGenerator::new(SEED);
    let mut world = World::default();
    for z in -VIEW_DISTANCE..=VIEW_DISTANCE {
        for x in -VIEW_DISTANCE..=VIEW_DISTANCE {
            let pos = ChunkPos::new(x, z);
            world.insert(pos, generator.generate(pos));
        }
    }
    log::info!(
        "generated {} chunks in {:?}",
        (2 * VIEW_DISTANCE + 1).pow(2),
        started.elapsed()
    );
    world
}

impl Game {
    fn set_mouse_grab(&mut self, grab: bool) {
        let mode = if grab {
            CursorGrabMode::Locked
        } else {
            CursorGrabMode::None
        };
        let ok = self.window.set_cursor_grab(mode).is_ok()
            || (grab
                && self
                    .window
                    .set_cursor_grab(CursorGrabMode::Confined)
                    .is_ok());
        self.window.set_cursor_visible(!grab);
        self.mouse_grabbed = grab && ok;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.game.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title("McRust")
            .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );
        let mut renderer =
            pollster::block_on(Renderer::new(window.clone())).expect("failed to init renderer");

        let world = build_world();
        let started = Instant::now();
        let mut triangles = 0;
        for pos in world.chunk_positions() {
            let mesh = mesh_chunk(&world, pos);
            triangles += mesh.indices.len() / 3;
            renderer.upload_mesh(&mesh);
        }
        log::info!("meshed {triangles} triangles in {:?}", started.elapsed());
        renderer.fog_end = (VIEW_DISTANCE * 16) as f32;

        let spawn_y = TerrainGenerator::new(SEED).surface_height(8, 8) as f32 + 30.0;
        let camera = Camera::new(Vec3::new(8.0, spawn_y, 8.0), renderer.aspect());
        let mut game = Game {
            window,
            renderer,
            camera,
            keys: HashSet::new(),
            mouse_grabbed: false,
            last_frame: Instant::now(),
            fps_timer: Instant::now(),
            frames: 0,
        };
        game.set_mouse_grab(true);
        self.game = Some(game);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(game) = self.game.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                game.renderer.resize(size.width, size.height);
                game.camera.aspect = game.renderer.aspect();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat,
                        ..
                    },
                ..
            } => match state {
                ElementState::Pressed => {
                    if code == KeyCode::Escape && !repeat {
                        game.set_mouse_grab(!game.mouse_grabbed);
                    }
                    game.keys.insert(code);
                }
                ElementState::Released => {
                    game.keys.remove(&code);
                }
            },
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } if !game.mouse_grabbed => {
                game.set_mouse_grab(true);
            }
            WindowEvent::Focused(false) => {
                game.keys.clear();
                game.set_mouse_grab(false);
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - game.last_frame).as_secs_f32().min(0.1);
                game.last_frame = now;
                game.camera.update(&game.keys, dt);
                if let Err(e) = game.renderer.render(&game.camera) {
                    log::error!("render error: {e:#}");
                    event_loop.exit();
                }
                game.frames += 1;
                if game.fps_timer.elapsed().as_secs_f32() >= 1.0 {
                    let p = game.camera.position;
                    game.window.set_title(&format!(
                        "McRust | {} fps | XYZ {:.1} / {:.1} / {:.1}",
                        game.frames, p.x, p.y, p.z
                    ));
                    game.frames = 0;
                    game.fps_timer = Instant::now();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let (Some(game), DeviceEvent::MouseMotion { delta }) = (self.game.as_mut(), event) {
            if game.mouse_grabbed {
                game.camera.rotate(delta.0, delta.1);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(game) = &self.game {
            game.window.request_redraw();
        }
    }
}
