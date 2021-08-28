use cpu::CPU;
use memory::Memory;

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::Read;
use std::io::SeekFrom;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;
use sdl2::EventPump;

fn main() {
    /* let sdl_context = sdl2::init().unwrap();
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
    let code = [
        // ARM red line code
        // 0x01, 0x13, 0xA0, 0xE3, 0x01, 0x0B, 0xA0, 0xE3, 0x03, 0x00, 0x80, 0xE3, 0xB0, 0x00, 0xC1,
        // 0xE1, 0x1F, 0x00, 0xA0, 0xE3, 0x06, 0x14, 0xA0, 0xE3, 0x96, 0x1C, 0x81, 0xE2, 0x00, 0x20,
        // 0xA0, 0xE3, 0xB0, 0x00, 0xC1, 0xE1, 0x02, 0x10, 0x81, 0xE2, 0x01, 0x20, 0x82, 0xE2, 0xF0,
        // 0x00, 0x52, 0xE3, 0xFA, 0xFF, 0xFF, 0x1A, 0xFE, 0xFF, 0xFF, 0xEA, 0x00, 0x00, 0x00, 0x00,
        // 0x00, 0x00, 0x00,
        // 0x00,
        // Thumb red line code
        0x01, 0x00, 0x8F, 0xE2, 0x10, 0xFF, 0x2F, 0xE1, 0x04, 0x21, 0x09, 0x06, 0x04, 0x20, 0x00,
        0x02, 0x03, 0x23, 0x18, 0x43, 0x08, 0x80, 0x1f, 0x20, 0x06, 0x21, 0x09, 0x06, 0x96, 0x23,
        0x1b, 0x02, 0xc9, 0x18, 0x00, 0x22, 0x08, 0x80, 0x02, 0x31, 0x01, 0x32, 0xf0, 0x2a, 0xfa,
        0xd1, 0xfe, 0xe7,
    ];
    // bios[..code.len()].clone_from_slice(&code);

    let mut bios_file = File::open(r"D:\Henry\dev\rust\mineral\gba_bios.bin").unwrap();
    bios_file.read(&mut bios).expect("buffer overflow");

    let mut cart = vec![0; 0x400000];
    // let mut cart_file =
    //     File::open(r"D:\Henry\ROMs\GBA\Super Dodge Ball Advance (USA).gba").unwrap();
    // let mut cart_file = File::open(r"D:\Henry\ROMs\GBA\Advanced Wars  # GBA.GBA").unwrap();
    let mut cart_file = File::open(r"D:\Henry\ROMs\GBA\tonc-bin\brin_demo.gba").unwrap();
    // let mut cart_file =
    //     File::open(r"D:\Henry\ROMs\GBA\test_roms\arm_wrestler\armwrestler.gba").unwrap();
    // let mut cart_file = File::open(r"D:\Henry\ROMs\GBA\test_roms\cpu_test\CPUTest.gba").unwrap();
    // let mut cart_file =
    //     File::open(r"D:\Henry\ROMs\GBA\test_roms\tonc_gba_demos\hello.gba").unwrap();
    // let mut cart_file =
    //     File::open(r"D:\Henry\ROMs\GBA\test_roms\tonc_gba_demos\cbb_demo.gba").unwrap();
    // let mut cart_file = File::open(r"D:\Henry\ROMs\GBA\gba-tests-master\arm\arm.gba").unwrap();
    // let mut cart_file = File::open(r"D:\Henry\ROMs\GBA\gba-tests-master\thumb\thumb.gba").unwrap();
    // let mut cart_file =
    //     File::open(r"D:\Henry\ROMs\GBA\Pokemon - Ruby Version (U) (V1.1).gba").unwrap();
    // let mut cart_file =
    //     File::open(r"D:\Henry\ROMs\GBA\test_roms\tonc_gba_demos\bigmap.gba").unwrap();
    // let mut cart_file = File::open(r"D:\Henry\ROMs\GBA\gba-tests-master\ppu\stripes.gba").unwrap();
    // let mut cart = vec![0; 0x800000];
    // let mut cart_file =
    //     File::open(r"D:\Henry\ROMs\Harvest Moon - Friends of Mineral Town (U) [!].gba").unwrap();
    // cart_file.seek(SeekFrom::Start(192)).unwrap();
    cart_file.read(&mut cart).expect("buffer overflow");

    let mut cpu = CPU::new();
    cpu.bios_rom.flash(bios);
    cpu.cart_rom.flash(cart);

    let mut screen_buff = [0u8; 240 * 160 * 2];
    let mut cycle_count: u64 = 0;
    let mut video_mode = 0;
    loop {
        cpu.tick();

        if cycle_count % 1_000_000 == 0 {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => std::process::exit(0),
                    Event::KeyDown {
                        keycode: Some(Keycode::S),
                        ..
                    } => {
                        video_mode = (video_mode + 1) % 6;
                    }
                    Event::KeyDown {
                        keycode: Some(Keycode::Return),
                        ..
                    } => cpu.start_button_press = true,
                    Event::KeyDown {
                        keycode: Some(Keycode::Up),
                        ..
                    } => cpu.up_button_press = true,
                    Event::KeyDown {
                        keycode: Some(Keycode::Down),
                        ..
                    } => cpu.down_button_press = true,
                    _ => {}
                }
            }

            const SCREEN_BLOCK: usize = 30;
            const CHAR_BLOCK: usize = 0;
            if video_mode == 0 {
                let map_base = SCREEN_BLOCK * 0x800;
                for tile_row in 0..20 {
                    for tile_col in 0..30 {
                        let map_entry =
                            cpu.vram.read_u16(map_base + 2 * (tile_row * 32 + tile_col));
                        let tile_n = map_entry & 0b11_1111_1111;
                        let flip_h = (map_entry >> 10) & 1 == 1;
                        let flip_v = (map_entry >> 11) & 1 == 1;
                        let palette_n = (map_entry >> 12) & 0b1111;

                        for row in 0..8 {
                            for byte_n in 0..4 {
                                let data = cpu.vram.read(
                                    0x4000 * CHAR_BLOCK + 32 * tile_n as usize + row * 4 + byte_n,
                                );
                                let color_i_left = data & 0b1111;
                                let color_left = cpu.palette_ram.read_u16(
                                    2 * (palette_n as usize * 16 + color_i_left as usize),
                                );
                                let color_i_right = (data >> 4) & 0b1111;
                                let color_right = cpu.palette_ram.read_u16(
                                    2 * (palette_n as usize * 16 + color_i_right as usize),
                                );
                                screen_buff
                                    [480 * (8 * tile_row + row) + 16 * tile_col + 4 * byte_n + 0] =
                                    color_left as u8;
                                screen_buff
                                    [480 * (8 * tile_row + row) + 16 * tile_col + 4 * byte_n + 1] =
                                    (color_left >> 8) as u8;
                                screen_buff
                                    [480 * (8 * tile_row + row) + 16 * tile_col + 4 * byte_n + 2] =
                                    color_right as u8;
                                screen_buff
                                    [480 * (8 * tile_row + row) + 16 * tile_col + 4 * byte_n + 3] =
                                    (color_right >> 8) as u8;
                            }
                        }
                    }
                }
            } else if video_mode == 3 {
                let mut pixel_i = 0;
                for y in 0..160 {
                    for x in 0..240 {
                        let color = cpu.vram.read_u16(2 * (x + 240 * y));
                        screen_buff[pixel_i + 0] = color as u8;
                        screen_buff[pixel_i + 1] = (color >> 8) as u8;
                        pixel_i += 2;
                    }
                }
            } else if video_mode == 4 {
                let mut pixel_i = 0;
                for y in 0..160 {
                    for x in 0..240 {
                        let color_i = cpu.vram.read(x + 240 * y);
                        let color = cpu.palette_ram.read_u16(2 * (color_i as usize));
                        screen_buff[pixel_i + 0] = color as u8;
                        screen_buff[pixel_i + 1] = (color >> 8) as u8;
                        pixel_i += 2;
                    }
                }
            }

            texture.update(None, &screen_buff, 240 * 2).unwrap();
            canvas.copy(&texture, None, None).unwrap();
            canvas.present();

            // use std::thread;
            // use std::time;
            // thread::sleep(time::Duration::from_millis(16));
        }

        cycle_count = cycle_count.wrapping_add(1);
    } */
}
