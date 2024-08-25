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

mod clipboard;
mod config;
mod gui;
mod tree_gl;

use crate::tree_gl::TreeGl;
use glow::HasContext;
use glutin::surface::SwapInterval;
use gui::{State, UiState};
//use imgui::ConfigFlags;
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
use imgui_winit_support::{
    winit::{
        dpi::LogicalSize,
        event_loop::EventLoop,
        event::{self, ElementState, MouseButton},
        window::{Window, WindowBuilder},
        event::{Event, WindowEvent},
        keyboard::{KeyCode, PhysicalKey},
    },
    WinitPlatform,
};

use raw_window_handle::HasRawWindowHandle;

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
    // Common setup for creating a winit window and imgui context, not specifc
    // to this renderer at all except that glutin is used to create the window
    // since it will give us access to a GL context
    let (event_loop, window, surface, context) = create_window();
    let (mut winit_platform, mut imgui_context) = imgui_init(&window);

    // OpenGL context from glow
    let gl = glow_context(&context);

    // OpenGL renderer from this crate
    let mut ig_renderer =
        imgui_glow_renderer::AutoRenderer::initialize(gl, &mut imgui_context).expect("failed to create renderer");

    let mut last_frame = Instant::now();
    let mut state = State::new(get_config());
    let mut vsync = state.config.vsync;
    set_vsync(&surface, &context, vsync);

    let mut tree_gl = TreeGl::default();
    tree_gl.init(ig_renderer.gl_context());
    window.set_visible(true);

    // Standard winit event loop
    let _ = event_loop.run(move |event, window_target| {
        // Consider making the line below work someday.
        // It suspends redrawing until there's an event.
        // Pretty good cpu/gpu savings.
        //*control_flow = glutin::event_loop::ControlFlow::Wait;
        match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                imgui_context.io_mut().update_delta_time(now.duration_since(last_frame));
                last_frame = now;
            }
            Event::AboutToWait => {
                winit_platform
                    .prepare_frame(imgui_context.io_mut(), &window)
                    .unwrap();
                window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // The renderer assumes you'll be clearing the buffer yourself
                unsafe { ig_renderer.gl_context().clear_color(0.0, 0.0, 0.05, 0.0) };
                unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT); };

                if state.request_regen {
                    tree_gl.regen_active(ig_renderer.gl_context(), &state.build.tree, &state.path_hovered, &state.path_red);
                    state.request_regen = false;
                }
                if state.request_recalc {
                    state.defence_calc = calc::calc_defence(&state.build);
                    state.request_recalc = false;
                }
                let ui = imgui_context.frame();
                match state.ui_state {
                    UiState::ChooseBuild => {
                        gui::build_selection::draw(ui, &mut state);
                        if state.show_settings {
                            gui::settings::draw(ui, &mut state);
                            if vsync != state.config.vsync {
                                vsync = state.config.vsync;
                                set_vsync(&surface, &context, vsync);
                            }
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
                            &state.build.tree,
                            ig_renderer.gl_context(),
                            state.zoom,
                            state.tree_translate,
                            &state.path_hovered,
                            &state.path_red,
                        );
                        gui::draw_top_panel(ui, &mut state);
                        gui::draw_left_panel(ui, &mut state);
                        gui::tree_view::draw(ui, &mut state);
                    }
                    _ => eprintln!("Can't draw state {:?}", state.ui_state),
                };
                if let Err(err) = process_state(&mut state) {
                    println!("State Error: {err}");
                    if state.ui_state == UiState::ImportBuild {
                        state.ui_state = UiState::ChooseBuild;
                    }
                }

                winit_platform.prepare_render(ui, &window);
                let draw_data = imgui_context.render();
                if let Err(err) = ig_renderer.render(draw_data) {
                    eprintln!("Error rendering imgui: {err}");
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
                state.last_instant = Instant::now();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                window_target.exit();
            }
            event => {
                let mut forward_event = true;
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
                            forward_event = false;
                            state.zoom = f32::max(0.50, state.zoom + v);
                        }
                    }
                    Event::WindowEvent {
                        event: WindowEvent::Resized(physical_size),
                        ..
                    } => {
                        unsafe {
                            state.dimensions = (physical_size.width, physical_size.height);
                            ig_renderer.gl_context().viewport(
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
                            } else {
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

                if forward_event {
                    winit_platform.handle_event(imgui_context.io_mut(), &window, &event);
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
        .with_window_builder(Some(window_builder))
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

fn imgui_init(window: &Window) -> (WinitPlatform, imgui::Context) {
    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut winit_platform = WinitPlatform::init(&mut imgui_context);
    winit_platform.attach_window(
        imgui_context.io_mut(),
        window,
        imgui_winit_support::HiDpiMode::Rounded,
    );

    imgui_context
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    if let Some(backend) = clipboard::init() {
        imgui_context.set_clipboard_backend(backend);
    } else {
        eprintln!("Failed to initialize clipboard");
    }

    imgui_context.io_mut().font_global_scale = (1.0 / winit_platform.hidpi_factor()) as f32;
    // For some reason imgui will register numpad 2,4,6,8 as navigation keys rather than the digits,
    // though this may be due to bad event passing
    //imgui_context.io_mut().config_flags |= ConfigFlags::NAV_ENABLE_KEYBOARD;

    (winit_platform, imgui_context)
}
