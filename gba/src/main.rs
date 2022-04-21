use gba::GBA;

use std::{env, fs::File, io::Read, time};

use sdl2::{event::Event, keyboard::Scancode, pixels::PixelFormatEnum};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: {} <GBA file>", args.get(0).unwrap(),);
        return;
    }

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem
        .window("Mineral", (240.0 * 3.0) as u32, (160.0 * 3.0) as u32)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().present_vsync().build().unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();
    canvas.set_scale(3.0, 3.0).unwrap();

    let creator = canvas.texture_creator();
    let mut texture = creator
        .create_texture_target(PixelFormatEnum::BGR555, 240, 160)
        .unwrap();

    let mut bios = vec![0; 0x4000];
    let mut bios_file = File::open(r"gba_bios.bin").unwrap();
    bios_file.read(&mut bios).expect("buffer overflow");

    let mut cart = vec![0; 0x800000 * 0xC];
    let mut cart_file = match File::open(args[1].clone()) {
        Ok(file) => file,
        Err(e) => {
            println!("error opening file: {}", e);
            return;
        }
    };
    cart_file.read(&mut cart).expect("buffer overflow");

    let mut gba = GBA::new();
    gba.flash_bios(bios);
    gba.flash_cart(cart);

    let controls = vec![
        Scancode::A,
        Scancode::S,
        Scancode::Down,
        Scancode::Up,
        Scancode::Left,
        Scancode::Right,
        Scancode::Return,
        Scancode::RShift,
        Scancode::Z,
        Scancode::X,
    ];

    let mut now = time::Instant::now();
    let mut frame_count = 0;
    let frames_per_rate_check = 60;
    let checks_per_rate_report = 2;
    let get_fps = |micros| (1f32 / ((micros / frames_per_rate_check) as f32 * 0.000001)) as u32;

    let mut fps_timer = time::Instant::now();
    loop {
        gba.tick();

        if let Some(framebuffer) = gba.try_get_framebuffer() {
            texture.update(None, &framebuffer, 240 * 2).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();

            if (frame_count + 1) % frames_per_rate_check == 0 {
                if (frame_count + 1) % (frames_per_rate_check * checks_per_rate_report) == 0 {
                    canvas
                        .window_mut()
                        .set_title(
                            &format!("Mineral | {} fps", get_fps(now.elapsed().as_micros()))[..],
                        )
                        .unwrap();
                }
                now = time::Instant::now();
            }
            frame_count += 1;

            let elapsed = fps_timer.elapsed();
            if elapsed < time::Duration::from_millis(16) {
                // thread::sleep(time::Duration::from_millis(16) - elapsed);
            }
            fps_timer = time::Instant::now();

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => std::process::exit(0),
                    _ => {}
                }
            }

            let kb_state = event_pump.keyboard_state();
            let controller_data = controls.iter().fold(0, |acc, control| {
                (acc << 1) | (!kb_state.is_scancode_pressed(*control)) as u16
            });

            gba.update_key_state(controller_data);
        }
    }
}
