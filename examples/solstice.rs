fn draw(source: &str) -> Result<solstice_2d::DrawList<'static>, eisenscript::Error> {
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

    use solstice_2d::Draw;
    let mut dl = solstice_2d::DrawList::default();
    dl.set_camera(solstice_2d::Transform3D::translation(0., -2., -5.));
    for (tx, primitive) in rules.iter() {
        use eisenscript::Primitive;
        let geometry = match primitive {
            Primitive::Box => solstice_2d::Box::new(1., 1., 1., 1, 1, 1),
            _ => unimplemented!(),
        };
        let color = tx_to_color(&tx);
        let tx = solstice_2d::Transform3D::translation(tx.x, tx.y, tx.z);
        dl.draw_with_color_and_transform(geometry, color, tx);
    }
    Ok(dl)
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

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("src.eis");
    let source = std::fs::read_to_string(&path).unwrap();
    let mut dl = draw(&source).unwrap_or_else(|err| {
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

    el.run(move |event, _el, cf| {
        use glutin::{
            event::{Event, WindowEvent},
            event_loop::*,
        };

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *cf = ControlFlow::Exit,
                _ => {}
            },
            Event::MainEventsCleared => window_ctx.window().request_redraw(),
            Event::RedrawRequested(_) => {
                if let Ok(src) = tx.try_recv() {
                    match draw(&src) {
                        Ok(new_dl) => dl = new_dl,
                        Err(err) => {
                            eprintln!("{}", err);
                        }
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
