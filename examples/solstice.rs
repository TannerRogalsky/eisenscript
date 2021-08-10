use eisenscript::Primitive;

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

    let source = "{ x -3 } 2 * { x 3 } 2 * { y 2 } 4 * { z -2 } box";
    let parser = eisenscript::Parser::new(eisenscript::Lexer::new(source));
    let rules = parser.rules().unwrap();

    let dl = {
        use solstice_2d::Draw;
        let mut dl = solstice_2d::DrawList::default();
        dl.set_camera(solstice_2d::Transform3D::translation(0., -2., -5.));
        for (tx, primitive) in rules.iter() {
            let geometry = match primitive {
                Primitive::Box => solstice_2d::Box::new(1., 1., 1., 1, 1, 1),
                _ => unimplemented!(),
            };
            let tx = solstice_2d::Transform3D::translation(tx.x, tx.y, tx.z);
            dl.draw_with_transform(geometry, tx);
        }
        dl
    };

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
