// todo: remove this once stable-ish
#![allow(dead_code)]

// Disable console on Windows in release builds
#![cfg_attr(
  all(
    target_os = "windows",
    not(debug_assertions),
  ),
  windows_subsystem = "windows"
)]

mod config;
mod gui;
mod tree_gl;

use crate::tree_gl::TreeGl;
use glow::HasContext;
use glutin::surface::SwapInterval;
use gui::{MainState, State, UiState};
use lightning_model::build::property;
use lightning_model::data::TREE;
use lightning_model::{build, util};
use std::error::Error;
use std::fs;
use std::ops::Neg;
use std::sync::Arc;
use std::{num::NonZeroU32, time::{Duration, Instant} };

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface},
};

use egui_glow::egui_winit::winit;
use raw_window_handle::HasWindowHandle;

use winit::{
    dpi::LogicalSize,
    event_loop::EventLoop,
    event::{self, ElementState, MouseButton, StartCause},
    window::{Window, WindowAttributes},
    event::WindowEvent,
    keyboard::Key,
};


const TITLE: &str = "Lightning";

fn process_state(state: &mut State) -> Result<(), Box<dyn Error>> {
    state.ui_state = match &state.ui_state {
        UiState::LoadBuild(path) => {
            state.build = util::load_build(path)?;
            state.level = state.build.property_int(property::Int::Level);
            state.request_recalc = true;
            println!("Loaded build from {}", &path.display());
            state.request_regen = true;
            UiState::Main(MainState::Tree)
        }
        #[cfg(feature = "import")]
        UiState::ImportBuild => {
            state.build = util::fetch_build(&state.import_account, &state.import_character)?;
            state.level = state.build.property_int(property::Int::Level);
            state.request_recalc = true;
            state.request_regen = true;
            println!("Fetched build: {} {}", &state.import_account, &state.import_character);
            UiState::Main(MainState::Tree)
        }
        UiState::NewBuild => {
            state.build = build::Build::new_player();
            state.level = state.build.property_int(property::Int::Level);
            state.request_recalc = true;
            state.request_regen = true;
            UiState::Main(MainState::Tree)
        }
        _ => state.ui_state.clone(),
    };
    Ok(())
}

fn round_to_nearest(f: f32, n: f32) -> f32 {
    (f / n).round() * n
}

fn get_config() -> config::Config {
    let path = config::config_dir().join("config.json");
    if let Ok(file) = fs::File::open(path) {
        if let Ok(config) = serde_json::from_reader(&file) {
            return config;
        }
    }
    config::Config::default()
}

fn set_vsync(surface: &Surface<WindowSurface>, context: &PossiblyCurrentContext, vsync: bool) {
    if vsync {
        if let Err(res) = surface.set_swap_interval(context, SwapInterval::Wait(NonZeroU32::new(1).unwrap())) {
            eprintln!("Error enabling vsync: {res:?}");
        }
    } else if let Err(res) = surface.set_swap_interval(context, SwapInterval::DontWait) {
        eprintln!("Error disabling vsync: {res:?}");
    }
}

struct GlowApp {
    window: Option<Window>,
    gl_context: Option<PossiblyCurrentContext>,
    gl_surface: Option<Surface<WindowSurface>>,
    gl: Option<Arc<glow::Context>>,
    egui_glow: Option<egui_glow::EguiGlow>,
    state: State,
    tree_gl: TreeGl,
}

impl GlowApp {
    fn new() -> Self {
        Self {
            window: None,
            gl_context: None,
            gl_surface: None,
            gl: None,
            egui_glow: None,
            state: State::new(get_config()),
            tree_gl: TreeGl::default(),
        }
    }
}

impl winit::application::ApplicationHandler<()> for GlowApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let (window, surface, context) = create_window(event_loop);
        let gl = std::sync::Arc::new(glow_context(&context));
        let egui_glow = egui_glow::EguiGlow::new(event_loop, gl.clone(), None, None, true);
        egui_glow.egui_ctx.style_mut(|style| {
            style.animation_time = 0.0;
            style.text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = 14.0;
            style.text_styles.get_mut(&egui::TextStyle::Button).unwrap().size = 14.0;
        });

        set_vsync(&surface, &context, self.state.config.vsync);

        if let Err(err) = config::create_config_builds_dir() {
            eprintln!("Error creating Lightning user directory: {err}");
        }

        self.tree_gl.init(&gl);
        self.tree_gl.regen_active(&gl, &self.state.build, &None, &None, None);
        window.set_visible(true);

        self.window = Some(window);
        self.gl_context = Some(context);
        self.gl_surface = Some(surface);
        self.gl = Some(gl);
        self.egui_glow = Some(egui_glow);
    }

    fn new_events(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        cause: winit::event::StartCause,
    ) {
        if let StartCause::ResumeTimeReached { .. } = &cause {
            self.window.as_mut().unwrap().request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let gl = self.gl.as_mut().unwrap();
        let mut state = &mut self.state;
        let window = self.window.as_mut().unwrap();
        let surface = self.gl_surface.as_mut().unwrap();
        let context = self.gl_context.as_mut().unwrap();
        let egui_glow = self.egui_glow.as_mut().unwrap();
        let tree_gl = &mut self.tree_gl;

        let mut vsync = state.config.vsync;

        match event {
            WindowEvent::RedrawRequested => {
                // The renderer assumes you'll be clearing the buffer yourself
                unsafe { gl.clear_color(0.0, 0.0, 0.05, 0.0) };
                unsafe { gl.clear(glow::COLOR_BUFFER_BIT); };

                if state.request_regen {
                    tree_gl.regen_active(&gl, &state.build, &state.path_hovered, &state.path_red, state.hovered_node);
                    state.request_regen = false;
                }
                if state.request_recalc {
                    state.recalc();
                }

                match state.ui_state.clone() {
                    UiState::ChooseBuild => {
                        egui_glow.run(&window, |egui_ctx| {
                            gui::build_selection::draw(egui_ctx, &mut state);
                            if state.show_settings {
                                gui::settings::draw(egui_ctx, &mut state);
                                if vsync != state.config.vsync {
                                    vsync = state.config.vsync;
                                    set_vsync(&surface, &context, vsync);
                                }
                            }
                        });
                        if state.ui_state != UiState::ChooseBuild {
                            window.request_redraw();
                        }
                    }
                    UiState::Main(main_state) => {
                        if main_state == MainState::Tree || matches!(main_state, MainState::ChooseMastery(_)) {
                            tree_gl.draw(
                                &gl,
                                state.zoom,
                                state.tree_translate,
                            );
                        }
                        egui_glow.run(&window, |egui_ctx| {
                            gui::panel::top::draw(egui_ctx, &mut state);
                            gui::panel::left::draw(egui_ctx, &mut state);
                            if main_state == MainState::Tree {
                                gui::tree_view::draw(egui_ctx, &mut state);
                            } else if main_state == MainState::Config {
                                gui::panel::config::draw(egui_ctx, &mut state);
                            } else if main_state == MainState::Skills {
                                gui::panel::skills::draw(egui_ctx, &mut state);
                            }
                            if let MainState::ChooseMastery(node_id) = main_state {
                                if let Some(effect) = gui::select_mastery_effect(egui_ctx, &state.build.tree.masteries, &TREE.nodes[&node_id]) {
                                    state.build.tree.masteries.insert(node_id, effect);
                                    state.ui_state = UiState::Main(MainState::Tree);
                                    state.request_recalc = true;
                                }
                            }
                            if state.show_settings {
                                gui::settings::draw(egui_ctx, &mut state);
                                if vsync != state.config.vsync {
                                    vsync = state.config.vsync;
                                    set_vsync(&surface, &context, vsync);
                                }
                            }
                        });
                    }
                    _ => {
                        eprintln!("Can't draw state {:?}", state.ui_state);
                        state.ui_state = UiState::ChooseBuild;
                    }
                };
                if let Err(err) = process_state(&mut state) {
                    eprintln!("State Error: {:?}: {}", state.ui_state, err);
                    if state.ui_state == UiState::ImportBuild {
                        state.ui_state = UiState::ChooseBuild;
                    }
                }

                egui_glow.paint(&window);
                if egui_glow.egui_ctx.has_requested_repaint() || state.request_recalc || state.request_regen {
                    window.request_redraw();
                }

                if !vsync {
                    let instant = Instant::now();
                    let overhead = (instant - state.last_instant).as_micros() as u64;
                    let sleep_for_us = 1000000 / state.config.framerate;
                    if overhead < sleep_for_us {
                        let sleep_for = Duration::from_micros(sleep_for_us - overhead);
                        std::thread::sleep(sleep_for);
                    }
                }
                if let Err(err) = surface.swap_buffers(&context) {
                    eprintln!("Failed to swap buffers: {err}");
                }
                state.redraw_counter += 1;
                state.last_instant = Instant::now();
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            event => {
                let egui_event_result = egui_glow.on_window_event(&window, &event);
                if egui_event_result.repaint {
                    window.request_redraw();
                }
                if !egui_event_result.consumed {
                    match event {
                        WindowEvent::MouseWheel {
                            delta: event::MouseScrollDelta::LineDelta(_h, v),
                            phase: event::TouchPhase::Moved,
                            ..
                        } => {
                            if state.ui_state == UiState::Main(MainState::Tree) && gui::is_over_tree(&state.mouse_pos) {
                                state.zoom_tmp = (state.zoom_tmp + v).clamp(1.0, 10.0);
                                state.zoom = state.zoom_tmp.round();
                                window.request_redraw();
                            }
                        }
                        WindowEvent::Resized(physical_size) => {
                            unsafe {
                                state.dimensions = (physical_size.width, physical_size.height);
                                gl.viewport(
                                    0,
                                    0,
                                    physical_size.width as i32,
                                    physical_size.height as i32,
                                );
                                window.request_redraw();
                            };
                        }
                        WindowEvent::MouseInput {
                            state: button_state,
                            button,
                            ..
                        } => {
                            if button == MouseButton::Left {
                                if button_state == ElementState::Pressed {
                                    if state.ui_state == UiState::Main(MainState::Tree) && gui::is_over_tree(&state.mouse_pos) {
                                        state.mouse_tree_drag = Some(state.mouse_pos);
                                        window.request_redraw();
                                    }
                                } else if button_state == ElementState::Released {
                                    if let Some(node) = state.hovered_node {
                                        state.snapshot();
                                        state.build.tree.flip_node(node.skill);
                                        if !state.build.tree.nodes.contains(&node.skill) {
                                            state.path_red = None;
                                            state.path_hovered = state.build.tree.find_path(node.skill);
                                        } else {
                                            if node.is_mastery {
                                                state.ui_state = UiState::Main(MainState::ChooseMastery(node.skill));
                                            }
                                            state.path_hovered = None;
                                        }
                                        let mut build_compare = state.build.clone();
                                        build_compare.tree.flip_node(node.skill);
                                        state.build_compare = Some(build_compare);
                                        state.request_regen = true;
                                        state.request_recalc = true;
                                        window.request_redraw();
                                    }
                                    state.mouse_tree_drag = None;
                                }
                            }
                        }
                        WindowEvent::KeyboardInput { event, .. } => {
                            if event.logical_key == Key::Character("z".into()) {
                                if state.modifiers.state().control_key() {
                                    state.undo();
                                }
                            }
                        }
                        WindowEvent::ModifiersChanged(modifiers) => {
                            state.modifiers = modifiers;
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            let (mut x, mut y) = (position.x as f32, position.y as f32);
                            state.mouse_pos = (x, y);
                            if state.ui_state == UiState::Main(MainState::Tree) && gui::is_over_tree(&state.mouse_pos) {
                                let aspect_ratio = state.dimensions.0 as f32 / state.dimensions.1 as f32;
                                if let Some(drag) = state.mouse_tree_drag {
                                    let (dx, dy) = (x - drag.0, y - drag.1);
                                    state.tree_translate.0 +=
                                        dx * 12500.0 / (state.dimensions.0 as f32 / 2.0) / (state.zoom / aspect_ratio);
                                    state.tree_translate.1 -=
                                        dy * 12500.0 / (state.dimensions.1 as f32 / 2.0) / state.zoom;
                                    state.mouse_tree_drag = Some(state.mouse_pos);
                                    state.hovered_node = None;
                                    window.request_redraw();
                                } else if gui::is_over_tree(&state.mouse_pos) {
                                    // There's gotta be simpler computations for this
                                    x -= state.dimensions.0 as f32 / 2.0;
                                    y -= state.dimensions.1 as f32 / 2.0;
                                    y = y.neg();
                                    x /= state.dimensions.0 as f32 / 2.0;
                                    y /= state.dimensions.1 as f32 / 2.0;
                                    x -= state.tree_translate.0 * (state.zoom / aspect_ratio) / 12500.0;
                                    y -= state.tree_translate.1 * state.zoom / 12500.0;
                                    x *= aspect_ratio;
                                    x *= 12500.0 / state.zoom;
                                    y *= 12500.0 / state.zoom;

                                    let hovered_node = tree_gl::hover::get_hovered_node(x, y);
                                    if hovered_node != state.hovered_node {
                                        state.hovered_node = hovered_node;
                                        if let Some(node) = hovered_node {
                                            let mut build_compare = state.build.clone();
                                            build_compare.tree.flip_node(node.skill);
                                            state.delta_compare = state.compare(&build_compare);
                                            if !state.build.tree.nodes.contains(&node.skill) {
                                                state.path_hovered = state.build.tree.find_path(node.skill);
                                                state.path_red = None;
                                            } else {
                                                state.path_red = Some(state.build.tree.find_path_remove(node.skill));
                                                state.path_hovered = None;
                                            }
                                        } else {
                                            state.path_red = None;
                                            state.path_hovered = None;
                                            state.delta_compare.clear();
                                            state.build_compare = None;
                                        }
                                        state.request_regen = true;
                                        window.request_redraw();
                                    }
                                } else if state.hovered_node.is_some() {
                                    state.hovered_node = None;
                                    state.path_hovered = None;
                                    state.build_compare = None;
                                    state.request_regen = true;
                                    window.request_redraw();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let mut app = GlowApp::new();
    event_loop.run_app(&mut app).expect("failed to run app");
}

fn create_window(event_loop: &winit::event_loop::ActiveEventLoop) -> (Window, Surface<WindowSurface>, PossiblyCurrentContext) {
    let window_builder = WindowAttributes::default()
        .with_title(TITLE)
        .with_inner_size(LogicalSize::new(1024, 768))
        .with_visible(false)
        .with_maximized(true);
    let (window, cfg) = glutin_winit::DisplayBuilder::new()
        .with_window_attributes(Some(window_builder.clone()))
        .build(event_loop, ConfigTemplateBuilder::new().with_multisampling(4), |mut configs| {
            configs.next().unwrap()
        })
        .expect("Failed to create OpenGL window");

    let window = window.unwrap();
    let context_attribs = ContextAttributesBuilder::new().build(Some(window.window_handle().unwrap().as_raw()));
    let context = unsafe {
        cfg.display()
            .create_context(&cfg, &context_attribs)
            .expect("Failed to create OpenGL context")
    };

    let surface_attribs = SurfaceAttributesBuilder::<WindowSurface>::new()
        .with_srgb(Some(true))
        .build(
            window.window_handle().unwrap().as_raw(),
            NonZeroU32::new(1024).unwrap(),
            NonZeroU32::new(768).unwrap(),
        );

    let surface = unsafe {
        cfg.display()
            .create_window_surface(&cfg, &surface_attribs)
            .expect("Failed to create OpenGL surface")
    };

    let context = context
        .make_current(&surface)
        .expect("Failed to make OpenGL context current");

    (window, surface, context)
}

fn glow_context(context: &PossiblyCurrentContext) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function_cstr(|s| context.display().get_proc_address(s).cast())
    }
}
