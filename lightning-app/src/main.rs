// todo: remove this once stable-ish
#![allow(dead_code)]

//! A basic self-contained example to get you from zero-to-demo-window as fast
//! as possible.

mod clipboard;
mod config;
mod gui;
mod tree_gl;

use crate::tree_gl::TreeGl;
use glow::HasContext;
use glutin::event::{self, ElementState, Event, MouseButton, VirtualKeyCode};
use glutin::{event_loop::EventLoop, WindowedContext};
use gui::{State, UiState};
use imgui::ConfigFlags;
use imgui_winit_support::WinitPlatform;
use lightning_model::{build, calc, util};
use std::error::Error;
use std::ops::Neg;
use std::time::Instant;

const TITLE: &str = "Lightning";

type Window = WindowedContext<glutin::PossiblyCurrent>;

fn process_state(state: &mut State) -> Result<(), Box<dyn Error>> {
    state.ui_state = match &state.ui_state {
        UiState::LoadBuild(path) => {
            state.build = util::load_build(path)?;
            state.request_recalc = true;
            println!("Loaded build from {}", &path.display());
            state.request_regen = true;
            UiState::Main
        }
        UiState::ImportBuild => {
            state.build = util::fetch_build(&state.import_account, &state.import_character)?;
            state.request_recalc = true;
            println!("Fetched build: {} {}", &state.import_account, &state.import_character);
            state.request_regen = true;
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

fn main() {
    // Common setup for creating a winit window and imgui context, not specifc
    // to this renderer at all except that glutin is used to create the window
    // since it will give us access to a GL context
    let (event_loop, window) = create_window();
    let (mut winit_platform, mut imgui_context) = imgui_init(&window);

    // OpenGL context from glow
    let gl = glow_context(&window);

    // OpenGL renderer from this crate
    let mut ig_renderer =
        imgui_glow_renderer::AutoRenderer::initialize(gl, &mut imgui_context).expect("failed to create renderer");

    let mut last_frame = Instant::now();

    let mut state = State::default();
    if let Err(err) = state.config.save() {
        eprintln!("Failed to save config: {err:?}");
    }

    let mut tree_gl = TreeGl::default();
    tree_gl.init(ig_renderer.gl_context());
    // Standard winit event loop
    event_loop.run(move |event, _, control_flow| {
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
            Event::MainEventsCleared => {
                winit_platform
                    .prepare_frame(imgui_context.io_mut(), window.window())
                    .unwrap();
                window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                // The renderer assumes you'll be clearing the buffer yourself
                unsafe { ig_renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };

                if state.request_regen {
                    tree_gl.regen_active(ig_renderer.gl_context(), &state.build.tree, &state.path_hovered);
                    state.request_regen = false;
                }
                if state.request_recalc {
                    state.defence_calc = calc::calc_defence(&state.build);
                    state.request_recalc = false;
                }
                let ui = imgui_context.frame();
                match state.ui_state {
                    UiState::ChooseBuild => gui::build_selection::draw(ui, &mut state),
                    UiState::Main => {
                        if let Some(node) = state.hovered_node {
                            if state.path_hovered.is_none() && !state.build.tree.nodes.contains(&node.skill) {
                                let path_hovered = state.build.tree.find_path(node.skill);
                                if state.path_hovered.is_none() && path_hovered.is_some() {
                                    state.request_regen = true;
                                }
                                state.path_hovered = path_hovered;
                            }
                        } else {
                            if state.path_hovered.is_some() {
                                state.request_regen = true;
                            }
                            state.path_hovered = None;
                        }
                        tree_gl.draw(
                            &state.build.tree,
                            ig_renderer.gl_context(),
                            state.zoom,
                            state.tree_translate,
                            &state.path_hovered,
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

                winit_platform.prepare_render(ui, window.window());
                let draw_data = imgui_context.render();
                ig_renderer.render(draw_data).expect("error rendering imgui");

                window.swap_buffers().unwrap();
            }
            Event::WindowEvent {
                event: glutin::event::WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = glutin::event_loop::ControlFlow::Exit;
            }
            event => {
                let mut forward_event = true;
                match event {
                    Event::WindowEvent {
                        event:
                            event::WindowEvent::MouseWheel {
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
                        event: event::WindowEvent::Resized(physical_size),
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
                            event::WindowEvent::MouseInput {
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
                                }
                                state.mouse_tree_drag = None;
                            }
                        }
                    }
                    Event::WindowEvent {
                        event:
                            event::WindowEvent::KeyboardInput {
                                input:
                                    event::KeyboardInput {
                                        virtual_keycode: Some(key),
                                        state: key_state,
                                        ..
                                    },
                                ..
                            },
                        ..
                    } => match key {
                        VirtualKeyCode::Left => state.key_left = key_state,
                        VirtualKeyCode::Right => state.key_right = key_state,
                        VirtualKeyCode::Up => state.key_up = key_state,
                        VirtualKeyCode::Down => state.key_down = key_state,
                        _ => {}
                    },
                    Event::WindowEvent {
                        event: event::WindowEvent::CursorMoved { position, .. },
                        ..
                    } => {
                        let (mut x, mut y) = (position.x as f32, position.y as f32);
                        state.mouse_pos = (x, y);
                        let aspect_ratio = state.dimensions.0 as f32 / state.dimensions.1 as f32;
                        if let Some(drag) = state.mouse_tree_drag {
                            let (dx, dy) = (x - drag.0, y - drag.1);
                            state.tree_translate.0 +=
                                (dx * 12500.0 / (state.dimensions.0 as f32 / 2.0) / (state.zoom / aspect_ratio)) as i32;
                            state.tree_translate.1 -=
                                (dy * 12500.0 / (state.dimensions.1 as f32 / 2.0) / state.zoom) as i32;
                            state.mouse_tree_drag = Some(state.mouse_pos);
                        } else if gui::is_over_tree(&state.mouse_pos) {
                            // There's gotta be simpler computations for this
                            x -= state.dimensions.0 as f32 / 2.0;
                            y -= state.dimensions.1 as f32 / 2.0;
                            y = y.neg();
                            x /= state.dimensions.0 as f32 / 2.0;
                            y /= state.dimensions.1 as f32 / 2.0;
                            x -= state.tree_translate.0 as f32 * (state.zoom / aspect_ratio) / 12500.0;
                            y -= state.tree_translate.1 as f32 * state.zoom / 12500.0;
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
                    winit_platform.handle_event(imgui_context.io_mut(), window.window(), &event);
                }
            }
        }
    });
}

fn create_window() -> (EventLoop<()>, Window) {
    let event_loop = glutin::event_loop::EventLoop::new();
    let window = glutin::window::WindowBuilder::new()
        .with_title(TITLE)
        .with_inner_size(glutin::dpi::LogicalSize::new(1280, 720));
    let window = glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_multisampling(4)
        .build_windowed(window, &event_loop)
        .expect("could not create window");
    let window = unsafe { window.make_current().expect("could not make window context current") };
    (event_loop, window)
}

fn glow_context(window: &Window) -> glow::Context {
    unsafe { glow::Context::from_loader_function(|s| window.get_proc_address(s).cast()) }
}

fn imgui_init(window: &Window) -> (WinitPlatform, imgui::Context) {
    let mut imgui_context = imgui::Context::create();
    imgui_context.set_ini_filename(None);

    let mut winit_platform = WinitPlatform::init(&mut imgui_context);
    winit_platform.attach_window(
        imgui_context.io_mut(),
        window.window(),
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
    imgui_context.io_mut().config_flags |= ConfigFlags::NAV_ENABLE_KEYBOARD;

    (winit_platform, imgui_context)
}
