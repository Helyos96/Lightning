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
use gui::{State, UiState};
use lightning_model::{build, calc, util};
use std::error::Error;
use std::fs;
use std::ops::Neg;
use std::{num::NonZeroU32, time::{Duration, Instant} };

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextAttributesBuilder, NotCurrentGlContext, PossiblyCurrentContext},
    display::{GetGlDisplay, GlDisplay},
    surface::{GlSurface, Surface, SurfaceAttributesBuilder, WindowSurface},
};

use egui_glow::egui_winit::winit;
use raw_window_handle::HasRawWindowHandle;

use winit::{
    dpi::LogicalSize,
    event_loop::{ControlFlow, EventLoop},
    event::{self, ElementState, MouseButton, StartCause},
    window::{Window, WindowBuilder},
    event::{Event, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};


const TITLE: &str = "Lightning";

fn process_state(state: &mut State) -> Result<(), Box<dyn Error>> {
    state.ui_state = match &state.ui_state {
        UiState::LoadBuild(path) => {
            state.build = util::load_build(path)?;
            state.request_recalc = true;
            println!("Loaded build from {}", &path.display());
            state.request_regen = true;
            UiState::Main
        }
        #[cfg(feature = "import")]
        UiState::ImportBuild => {
            state.build = util::fetch_build(&state.import_account, &state.import_character)?;
            state.request_recalc = true;
            state.request_regen = true;
            println!("Fetched build: {} {}", &state.import_account, &state.import_character);
            UiState::Main
        }
        UiState::NewBuild => {
            state.build = build::Build::new_player();
            state.request_recalc = true;
            UiState::Main
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
    } else {
        if let Err(res) = surface.set_swap_interval(context, SwapInterval::DontWait) {
            eprintln!("Error disabling vsync: {res:?}");
        }
    }
}

fn main() {
    let (event_loop, window, surface, context) = create_window();
    let gl = std::sync::Arc::new(glow_context(&context));
    let mut egui_glow = egui_glow::EguiGlow::new(&event_loop, gl.clone(), None, None);
    egui_glow.egui_ctx.style_mut(|style| {
        style.animation_time = 0.0;
        style.text_styles.get_mut(&egui::TextStyle::Body).unwrap().size = 14.0;
        style.text_styles.get_mut(&egui::TextStyle::Button).unwrap().size = 14.0;
    });

    let mut state = State::new(get_config());
    let mut vsync = state.config.vsync;
    set_vsync(&surface, &context, vsync);

    if let Err(err) = config::create_config_builds_dir() {
        eprintln!("Error creating Lightning user directory: {err}");
    }

    let mut tree_gl = TreeGl::default();
    tree_gl.init(&gl);
    tree_gl.regen_active(&gl, &state.build, &None, &None);
    window.set_visible(true);

    // Standard winit event loop
    let _ = event_loop.run(move |event, window_target| {
        window_target.set_control_flow(ControlFlow::WaitUntil(Instant::now() + Duration::from_millis(50)));
        match event {
            Event::NewEvents(ne) => {
                if matches!(ne, StartCause::ResumeTimeReached{..}) {
                    window.request_redraw();
                }
            }
            Event::AboutToWait => {
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // The renderer assumes you'll be clearing the buffer yourself
                unsafe { gl.clear_color(0.0, 0.0, 0.05, 0.0) };
                unsafe { gl.clear(glow::COLOR_BUFFER_BIT); };

                if state.request_regen {
                    tree_gl.regen_active(&gl, &state.build, &state.path_hovered, &state.path_red);
                    state.request_regen = false;
                }
                if state.request_recalc {
                    state.defence_calc = calc::calc_defence(&state.build);
                    state.request_recalc = false;
                }

                match state.ui_state {
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
                    UiState::Main => {
                        if let Some(node) = state.hovered_node {
                            if !state.build.tree.nodes.contains(&node.skill) {
                                if state.path_hovered.is_none() {
                                    let path_hovered = state.build.tree.find_path(node.skill);
                                    if state.path_hovered.is_none() && path_hovered.is_some() {
                                        state.request_regen = true;
                                    }
                                    state.path_hovered = path_hovered;
                                    state.path_red = None;
                                }
                            } else {
                                if state.path_red.is_none() {
                                    let path_red = state.build.tree.find_path_remove(node.skill);
                                    state.request_regen = true;
                                    state.path_hovered = None;
                                    state.path_red = Some(path_red);
                                }
                            }
                        } else {
                            if state.path_hovered.is_some() || state.path_red.is_some() {
                                state.request_regen = true;
                            }
                            state.path_hovered = None;
                            state.path_red = None;
                        }

                        tree_gl.draw(
                            &gl,
                            state.zoom,
                            state.tree_translate,
                        );
                        egui_glow.run(&window, |egui_ctx| {
                            gui::draw_top_panel(egui_ctx, &mut state);
                            gui::draw_left_panel(egui_ctx, &mut state);
                            gui::tree_view::draw(egui_ctx, &mut state);
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
                state.last_instant = Instant::now();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                window_target.exit();
            }
            event => {
                window.request_redraw();
                let mut process_event = true;
                if let Event::WindowEvent {event, .. } = &event {
                    process_event = !egui_glow.on_window_event(&window, event).consumed;
                }
                if process_event {
                    match event {
                        Event::WindowEvent {
                            event:
                                WindowEvent::MouseWheel {
                                    delta: event::MouseScrollDelta::LineDelta(_h, v),
                                    phase: event::TouchPhase::Moved,
                                    ..
                                },
                            ..
                        } => {
                            if state.ui_state == UiState::Main && gui::is_over_tree(&state.mouse_pos) {
                                state.zoom_tmp = (state.zoom_tmp + v).clamp(1.0, 10.0);
                                state.zoom = state.zoom_tmp.round();
                            }
                        }
                        Event::WindowEvent {
                            event: WindowEvent::Resized(physical_size),
                            ..
                        } => {
                            unsafe {
                                state.dimensions = (physical_size.width, physical_size.height);
                                gl.viewport(
                                    0,
                                    0,
                                    physical_size.width as i32,
                                    physical_size.height as i32,
                                )
                            };
                        }
                        Event::WindowEvent {
                            event:
                                WindowEvent::MouseInput {
                                    state: button_state,
                                    button,
                                    ..
                                },
                            ..
                        } => {
                            if button == MouseButton::Left {
                                if button_state == ElementState::Pressed {
                                    if state.ui_state == UiState::Main && gui::is_over_tree(&state.mouse_pos) {
                                        state.mouse_tree_drag = Some(state.mouse_pos);
                                    }
                                } else if button_state == ElementState::Released {
                                    if state.hovered_node.is_some() {
                                        state.build.tree.flip_node(state.hovered_node.as_ref().unwrap().skill);
                                        state.request_regen = true;
                                        state.request_recalc = true;
                                        state.path_hovered = None;
                                        state.path_red = None;
                                    }
                                    state.mouse_tree_drag = None;
                                }
                            }
                        }
                        Event::WindowEvent {
                            event:
                                WindowEvent::KeyboardInput {
                                    event:
                                        event::KeyEvent {
                                            physical_key: key,
                                            state: key_state,
                                            ..
                                        },
                                    ..
                                },
                            ..
                        } => match key {
                            PhysicalKey::Code(KeyCode::ArrowLeft) => state.key_left = key_state,
                            PhysicalKey::Code(KeyCode::ArrowRight) => state.key_right = key_state,
                            PhysicalKey::Code(KeyCode::ArrowUp) => state.key_up = key_state,
                            PhysicalKey::Code(KeyCode::ArrowDown) => state.key_down = key_state,
                            _ => {}
                        },
                        Event::WindowEvent {
                            event: WindowEvent::CursorMoved { position, .. },
                            ..
                        } => {
                            let (mut x, mut y) = (position.x as f32, position.y as f32);
                            state.mouse_pos = (x, y);
                            let aspect_ratio = state.dimensions.0 as f32 / state.dimensions.1 as f32;
                            if let Some(drag) = state.mouse_tree_drag {
                                let (dx, dy) = (x - drag.0, y - drag.1);
                                state.tree_translate.0 +=
                                    dx * 12500.0 / (state.dimensions.0 as f32 / 2.0) / (state.zoom / aspect_ratio);
                                state.tree_translate.1 -=
                                    dy * 12500.0 / (state.dimensions.1 as f32 / 2.0) / state.zoom;
                                state.mouse_tree_drag = Some(state.mouse_pos);
                                state.hovered_node = None;
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

                                state.hovered_node = tree_gl::hover::get_hovered_node(x, y);
                            } else if state.hovered_node.is_some() {
                                state.hovered_node = None;
                                state.path_hovered = None;
                                state.request_regen = true;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    });
}

fn create_window() -> (EventLoop<()>, Window, Surface<WindowSurface>, PossiblyCurrentContext) {
    let event_loop = EventLoop::new().unwrap();
    let window_builder = WindowBuilder::new()
        .with_title(TITLE)
        .with_inner_size(LogicalSize::new(1024, 768))
        .with_visible(false);
    let (window, cfg) = glutin_winit::DisplayBuilder::new()
        .with_window_builder(Some(window_builder.clone()))
        .build(&event_loop, ConfigTemplateBuilder::new().with_multisampling(4), |mut configs| {
            configs.next().unwrap()
        })
        .expect("Failed to create OpenGL window");

    let window = window.unwrap();
    let context_attribs = ContextAttributesBuilder::new().build(Some(window.raw_window_handle()));
    let context = unsafe {
        cfg.display()
            .create_context(&cfg, &context_attribs)
            .expect("Failed to create OpenGL context")
    };

    let surface_attribs = SurfaceAttributesBuilder::<WindowSurface>::new()
        .with_srgb(Some(true))
        .build(
            window.raw_window_handle(),
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

    (event_loop, window, surface, context)
}

fn glow_context(context: &PossiblyCurrentContext) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function_cstr(|s| context.display().get_proc_address(s).cast())
    }
}
