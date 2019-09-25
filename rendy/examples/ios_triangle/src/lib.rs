mod graphics;

use graphics::Graphics;
use rendy::wsi::winit::{
    self,
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn run_game() -> Result<(), String> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("ios-triangle")
        .build(&event_loop)
        .map_err(|e| e.to_string())?;

    let mut graphics = Graphics::new(&window);

    event_loop.run(move |event, _window_target, control_flow| {
        match event {
            Event::EventsCleared => {
                // Render
                graphics.maintain();
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            Event::LoopDestroyed => {
                *control_flow = ControlFlow::Exit;
            }
            Event::Suspended => {}
            Event::Resumed => {}
            Event::WindowEvent {
                event: WindowEvent::RedrawRequested,
                ..
            } => {
                // Redraw the application.
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::KeyboardInput { input, .. },
                ..
            } => if let Some(key_code) = input.virtual_keycode {},
            event => {
                // println!("Got an event: {:#?}", event);
                *control_flow = ControlFlow::Poll;
            }
        }
    });
}

#[no_mangle]
pub extern "C" fn run() {
    match run_game() {
        Ok(_) => {}
        Err(e) => {
            println!("Error running game: {:#?}", e);
        }
    }
}
