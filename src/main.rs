use image::math::Rect;
use image::{open, GenericImage, GrayImage, ImageBuffer, Luma, Pixel, Primitive};

mod cli;

const fn num_bits<T>() -> usize {
    std::mem::size_of::<T>() * 8
}

fn log_2(x: u32) -> u32 {
    assert!(x > 0);
    num_bits::<u32>() as u32 - x.leading_zeros() - 1
}

fn apply2<F>(img1: &mut GrayImage, img2: &GrayImage, func: F)
where
    F: Fn(u8, u8) -> u8,
{
    for (x, y, p) in img1.enumerate_pixels_mut() {
        *p = Luma([func(p[0], img2.get_pixel(x, y)[0])])
    }
}

fn apply3<F>(img1: &mut GrayImage, img2: &GrayImage, img3: &GrayImage, func: F)
where
    F: Fn(u8, u8, u8) -> u8,
{
    for (x, y, p) in img1.enumerate_pixels_mut() {
        *p = Luma([func(p[0], img2.get_pixel(x, y)[0], img3.get_pixel(x, y)[0])])
    }
}

fn apply_with_offset<F>(img: &mut GrayImage, dx: u32, dy: u32, func: F)
where
    F: Fn(u8, u8) -> u8,
{
    let (width, height) = img.dimensions();
    for y in 0..height - dy {
        for x in 0..width - dx {
            let original_pixel = img.get_pixel(x, y)[0];
            img.put_pixel(
                x,
                y,
                Luma([func(original_pixel, img.get_pixel(x + dx, y + dy)[0])]),
            )
        }
    }
}

fn extreme_around<F>(img: &mut GrayImage, offset: u32, compare: &F)
where
    F: Fn(u8, u8) -> u8,
{
    apply_with_offset(img, offset, 0, compare);
    apply_with_offset(img, 0, offset, compare);
}

fn subtract(img1: &mut GrayImage, img2: &GrayImage) {
    apply2(img1, img2, |a, b| a.saturating_sub(b))
}

fn stretch<P, S>(img: &mut ImageBuffer<P, Vec<S>>, border: u32)
where
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    let (width, height) = img.dimensions();
    let (area_width, area_height) = (width - 2 * border, height - 2 * border);
    let area = Rect {
        x: 0,
        y: 0,
        width: area_width,
        height: area_height,
    };
    img.copy_within(area, border, border);
    let top = Rect {
        x: 0,
        y: border,
        width,
        height: 1,
    };
    let bottom = Rect {
        x: 0,
        y: border + area_height - 1,
        width,
        height: 1,
    };
    let left = Rect {
        x: border,
        y: 0,
        width: 1,
        height,
    };
    let right = Rect {
        x: border + area_width - 1,
        y: 0,
        width: 1,
        height,
    };
    for y in 0..border {
        img.copy_within(top, 0, y);
        img.copy_within(bottom, 0, height - y - 1);
    }
    for x in 0..border {
        img.copy_within(left, x, 0);
        img.copy_within(right, width - x - 1, 0);
    }
}

fn stretch_pixel_brightness(value: u8, black: u8, range: u8) -> u8 {
    255.min(255 * value.saturating_sub(black) as u32 / range as u32) as u8
}

fn equalize(img: &mut GrayImage, darkest: GrayImage, color_range: GrayImage, debug_mode: bool) {
    save_debug_image(&img, "corrected.unequalized.png".to_string(), debug_mode);
    apply3(img, &darkest, &color_range, |img_pixel, black, range_pixel| {
        stretch_pixel_brightness(img_pixel, black, range_pixel)
    })
}

fn save_debug_image(img: &GrayImage, name: String, debug_mode: bool) -> () {
    if debug_mode {
        img.save(format!("/tmp/{}", name)).unwrap();
    }
}

fn brighter(a: u8, b: u8) -> u8 {
    a.max(b)
}

fn darker(a: u8, b: u8) -> u8 {
    a.min(b)
}

fn histogram(img: &GrayImage) -> [u32; 256] {
    let mut result = [0; 256];
    for p in img.pixels() {
        result[p[0] as usize] += 1;
    }
    for i in 0..256 {
        result[i] = ((result[i] as f32) + 1f32).log(1.1) as u32
    }
    result
}

fn smooth(histogram: &mut [u32; 256], right_to_left: bool) -> Vec<u8> {
    let mut prev_direction = 0;
    let mut turns = vec![];
    for i in 0..255 {
        let (prev_index, index, next_index) = if !right_to_left {
            (i - 1, i, i + 1)
        } else {
            (256 - i, 255 - i, 254 - i)
        };
        histogram[index] = (histogram[index] + histogram[next_index]) / 2;
        if i == 0 {
            continue;
        };
        let direction = i64::signum(histogram[index] as i64 - histogram[prev_index] as i64);
        if direction != 0 && direction != prev_direction {
            if prev_direction != 0 {
                turns.push(index as u8);
            }
            prev_direction = direction
        }
    }
    if right_to_left {
        turns.reverse()
    }
    turns
}

fn get_turns(img: &GrayImage, debug_mode: bool) -> (u8, u8) {
    let mut h = histogram(&img);
    let mut round = 0;
    let turns = loop {
        let turns = smooth(&mut h, round % 2 == 1);
        if debug_mode {
            println!("Turns after smoothing: {:?}", turns)
        }
        round += 1;
        if turns.len() <= 3 || round > 100 {
            break turns;
        }
    };
    if turns.len() < 2 {
        (0, 255)
    } else {
        let dark = 2 * turns[0];
        let light = 255 - 2 * (255 - turns[turns.len() - 1]);
        if dark > light { (0, 255) } else { (dark, light) }
    }
}

fn main() {
    let args = cli::parse_args();
    for f in args.in_file_path {
        let img = open(&f).unwrap().grayscale().to_luma();
        save_debug_image(&img, format!("darkest.0.png"), args.debug);
        let (width, height) = img.dimensions();
        let smaller_extent = width.min(height);
        let max_rounds = log_2(smaller_extent) - 1;
        let bright_rounds = max_rounds - 2;
        let dark_rounds = max_rounds;
        let mut darkest = img.clone();
        extreme_around(&mut darkest, 1, &darker);
        save_debug_image(&darkest, format!("darkest.0.png"), args.debug);
        let mut brightest = darkest.clone();
        for round in 1..bright_rounds.max(dark_rounds) {
            let offset = 2u32.pow(round);
            if round < bright_rounds {
                extreme_around(&mut brightest, offset, &brighter);
            }
            save_debug_image(&brightest, format!("brightest.{}.png", offset), args.debug);
            if round < dark_rounds {
                extreme_around(&mut darkest, offset, &darker)
            }
            save_debug_image(&darkest, format!("darkest.{}.png", offset), args.debug);
        }
        stretch(&mut darkest, 2u32.pow(dark_rounds - 1));
        save_debug_image(
            &darkest,
            format!("darkest.stretched.{}.png", 2u32.pow(dark_rounds - 1)),
            args.debug,
        );
        stretch(&mut brightest, 2u32.pow(bright_rounds - 1));
        save_debug_image(
            &brightest,
            format!("brightest.stretched.{}.png", 2u32.pow(bright_rounds - 1)),
            args.debug,
        );

        let mut color_range = brightest;
        subtract(&mut color_range, &darkest);
        save_debug_image(&color_range, "color_range.png".to_string(), args.debug);

        let mut corrected = img;
        equalize(&mut corrected, darkest, color_range, args.debug);
        let (dark, light) = get_turns(&corrected, args.debug);
        if light > dark && dark > 0 && light < 255 {
            let range = light - dark;
            if args.debug {
                println!("{}..{} range is {}", dark, light, range)
            }
            for p in corrected.pixels_mut() {
                *p = Luma([stretch_pixel_brightness(p[0], dark, range)])
            }
        }
        corrected.save(cli::get_out_fname(&f)).unwrap();
    }
}
