use glutin::event::DeviceEvent;

fn draw<'a>(
    source: &'a str,
    assets: &Assets,
    camera: &solstice_2d::Transform3D,
) -> Result<solstice_2d::DrawList<'static>, eisenscript::Error<'a>> {
    let parser = eisenscript::Parser::new(eisenscript::Lexer::new(source));
    let rules = parser.rules()?;

    fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
        let i = (h * 6.).floor();
        let f = (h * 6.) - i;
        let p = v * (1. - s);
        let q = v * (1. - f * s);
        let t = v * (1. - (1. - f) * s);
        match i as usize % 6 {
            0 => [v, t, p],
            1 => [q, v, p],
            2 => [p, v, t],
            3 => [p, q, v],
            4 => [t, p, v],
            5 => [v, p, q],
            _ => panic!(),
        }
    }

    fn tx_to_color(tx: &eisenscript::Transform) -> solstice_2d::Color {
        let [r, g, b] = hsv_to_rgb((tx.hue % 360.) / 360., tx.sat, tx.brightness);
        solstice_2d::Color::new(r, g, b, tx.alpha)
    }

    let mut rng: rand::rngs::SmallRng = rand::SeedableRng::seed_from_u64(0);
    use solstice_2d::Draw;
    let mut dl = solstice_2d::DrawList::default();
    dl.set_camera(*camera);
    dl.set_shader(Some({
        let mut shader = assets.shader.clone();
        shader.send_uniform(
            "lightPos",
            mint::Vector3::from(camera.inverse_transform_point(0., 0., 0.)),
        );
        shader
    }));
    for (tx, primitive) in rules.iter(&mut eisenscript::ContextMut::new(&mut rng)) {
        use eisenscript::Primitive;
        let geometry = match primitive {
            Primitive::Box => solstice_2d::Box::new(1., 1., 1., 1, 1, 1),
            _ => unimplemented!(),
        };
        let color = tx_to_color(&tx);
        dl.draw_with_color_and_transform(geometry, color, tx);
    }

    dl.set_shader(Some({
        let mut shader = assets.plane.clone();
        shader.send_uniform("near", 0.01);
        shader.send_uniform("far", 1000.);
        shader
    }));
    dl.draw_with_color(solstice_2d::Plane::new(1., 1., 1, 1), [1., 0., 0., 1.]);
    Ok(dl)
}

struct Assets {
    shader: solstice_2d::Shader,
    plane: solstice_2d::Shader,
}

fn main() {
    let (width, height) = (1280., 720.);

    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_inner_size(glutin::dpi::PhysicalSize::new(width, height));
    let window_ctx = glutin::ContextBuilder::new()
        .with_vsync(true)
        .build_windowed(wb, &el)
        .unwrap();
    let window_ctx = unsafe { window_ctx.make_current() }.unwrap();
    let glow_ctx = unsafe {
        solstice_2d::solstice::glow::Context::from_loader_function(|addr| {
            window_ctx.get_proc_address(addr) as *const _
        })
    };
    let mut ctx = solstice_2d::solstice::Context::new(glow_ctx);
    let mut gfx = solstice_2d::Graphics::new(&mut ctx, width, height).unwrap();

    let root_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let shader = {
        let src = std::fs::read_to_string(root_path.join("examples").join("main.glsl")).unwrap();
        solstice_2d::Shader::with(&src, &mut ctx).unwrap()
    };

    let plane = {
        let src = std::fs::read_to_string(root_path.join("examples").join("plane.glsl")).unwrap();
        solstice_2d::Shader::with(&src, &mut ctx).unwrap()
    };

    let assets = Assets { shader, plane };
    let mut camera = solstice_2d::Transform3D::translation(0., -2., -5.);

    let path = root_path.join("examples").join("src.eis");
    let mut source = std::fs::read_to_string(&path).unwrap();
    let mut dl = draw(&source, &assets, &camera).unwrap_or_else(|err| {
        eprintln!("{}", err);
        solstice_2d::DrawList::default()
    });

    let (sx, tx) = std::sync::mpsc::channel();
    let mut watcher =
        notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
            Ok(event) => {
                for path in event.paths {
                    match std::fs::read_to_string(path) {
                        Ok(src) => {
                            if let Err(err) = sx.send(src) {
                                eprintln!("{}", err);
                            }
                        }
                        Err(err) => eprintln!("{}", err),
                    }
                }
            }
            Err(err) => eprintln!("{}", err),
        })
        .unwrap();
    notify::Watcher::watch(&mut watcher, &path, notify::RecursiveMode::NonRecursive).unwrap();

    #[derive(Default)]
    struct KeyState {
        w: bool,
        a: bool,
        s: bool,
        d: bool,
    }
    let mut keys = KeyState::default();

    enum MouseButtonState {
        Up,
        Down { start: [f32; 2] },
    }
    struct MouseState {
        position: [f32; 2],
        button: MouseButtonState,
    }
    let mut mouse = MouseState {
        position: [0., 0.],
        button: MouseButtonState::Up,
    };

    el.run(move |event, _el, cf| {
        use glutin::{
            event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
            event_loop::*,
        };

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *cf = ControlFlow::Exit,
                WindowEvent::MouseInput { state, button, .. } => {
                    if let MouseButton::Left = button {
                        match state {
                            ElementState::Pressed => {
                                mouse.button = MouseButtonState::Down {
                                    start: mouse.position,
                                }
                            }
                            ElementState::Released => mouse.button = MouseButtonState::Up,
                        }
                    }
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            state,
                            virtual_keycode: Some(virtual_keycode),
                            ..
                        },
                    ..
                } => {
                    let pressed = match state {
                        ElementState::Pressed => true,
                        ElementState::Released => false,
                    };
                    match virtual_keycode {
                        VirtualKeyCode::W => keys.w = pressed,
                        VirtualKeyCode::A => keys.a = pressed,
                        VirtualKeyCode::S => keys.s = pressed,
                        VirtualKeyCode::D => keys.d = pressed,
                        _ => {}
                    }
                }
                _ => {}
            },
            Event::DeviceEvent { event, .. } => match event {
                DeviceEvent::MouseMotion { delta: (dx, dy) } => {
                    let glutin::dpi::PhysicalSize { width, height } =
                        window_ctx.window().inner_size().cast::<f32>();
                    let arcball = |x: f32, y: f32| -> [f32; 3] {
                        let [px, py] = [x / width * 2. - 1., y / height * 2. - 1.];
                        let py = -py;
                        let squared = px * px + py * py;
                        if squared <= 1. {
                            [px, py, (1. - squared).sqrt()]
                        } else {
                            nalgebra::Vector3::new(px, py, 0.).normalize().into()
                        }
                    };

                    let [mx, my] = &mut mouse.position;
                    *mx += dx as f32;
                    *my += dy as f32;

                    if let MouseButtonState::Down { .. } = &mouse.button {
                        use solstice_2d::Rad as R;
                        camera *= solstice_2d::Transform3D::rotation(
                            R(dx as f32 / 100.),
                            R(dy as f32 / 100.),
                            R(0.),
                        );
                    }
                }
                _ => {}
            },
            Event::MainEventsCleared => window_ctx.window().request_redraw(),
            Event::RedrawRequested(_) => {
                if let Ok(src) = tx.try_recv() {
                    source = src;
                }

                let speed = 1.;
                if keys.w {
                    camera *= solstice_2d::Transform3D::translation(0., 0., speed)
                }
                if keys.s {
                    camera *= solstice_2d::Transform3D::translation(0., 0., -speed)
                }
                if keys.a {
                    camera *= solstice_2d::Transform3D::translation(speed, 0., 0.)
                }
                if keys.d {
                    camera *= solstice_2d::Transform3D::translation(-speed, 0., 0.)
                }

                match draw(&source, &assets, &camera) {
                    Ok(new_dl) => dl = new_dl,
                    Err(err) => {
                        eprintln!("{}", err);
                    }
                }

                ctx.clear();
                gfx.process(&mut ctx, &dl);
                window_ctx
                    .swap_buffers()
                    .expect("terrible, terrible damage");
            }
            _ => {}
        }
    })
}
