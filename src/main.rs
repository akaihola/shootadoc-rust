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

fn apply2<F>(img1: &mut GrayImage, img2: &GrayImage, func: &mut F)
where
    F: FnMut(u8, u8) -> u8,
{
    for (x, y, p) in img1.enumerate_pixels_mut() {
        *p = Luma([func(p[0], img2.get_pixel(x, y)[0])])
    }
}

fn apply_with_offset<F>(img: &mut GrayImage, dx: u32, dy: u32, func: &mut F)
where
    F: FnMut(u8, u8) -> u8,
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

fn extreme<F>(img: &mut GrayImage, dx: u32, dy: u32, compare: &mut F)
where
    F: FnMut(u8, u8) -> u8,
{
    apply_with_offset(img, dx, dy, compare)
}

fn extreme_around<F>(img: &mut GrayImage, offset: u32, compare: &mut F)
where
    F: FnMut(u8, u8) -> u8,
{
    extreme(img, offset, 0, compare);
    extreme(img, 0, offset, compare);
}

fn subtract(img1: &mut GrayImage, img2: &GrayImage) {
    apply2(img1, img2, &mut |a, b| a.saturating_sub(b))
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

fn equalize(img: &mut GrayImage, color_range: GrayImage) {
    apply2(img, &color_range, &mut |img_pixel, range_pixel| {
        0u32.max(255u32.min(255u32 * img_pixel as u32 / range_pixel as u32)) as u8
    })
}

fn save_debug_image(img: &GrayImage, name: String, debug_mode: bool) -> () {
    if debug_mode {
        img.save(format!("/tmp/{}", name)).unwrap();
    }
}

fn get_min_max_brightness(frequencies: [u32; 256], num_pixels: u32) -> (u8, u8) {
    let mut bright_count = num_pixels / 3;
    let mut dark_count = num_pixels / 100;
    let mut min_brightness = 0u8;
    let mut max_brightness = 255u8;
    for x in 0u8..255u8 {
        bright_count = bright_count.saturating_sub(frequencies[255 - x as usize]);
        if bright_count == 0 {
            max_brightness = 255 - x;
        }
        dark_count = dark_count.saturating_sub(frequencies[x as usize]);
        if dark_count == 0 {
            min_brightness = x;
        }
        println!(
            "Finding {} extremes {}..{}",
            x, min_brightness, max_brightness
        );
        if bright_count == 0 && dark_count == 0 {
            break;
        }
    }
    (min_brightness, max_brightness)
}

fn main() {
    let args = cli::parse_args();
    for f in args.in_file_path {
        let mut img = open(&f).unwrap().grayscale().to_luma();
        save_debug_image(&img, format!("darkest.0.png"), args.debug);
        let mut color_range = img.clone();
        let mut darkest = color_range.clone();
        let (width, height) = img.dimensions();
        let smaller_extent = width.min(height);
        let num_pixels = width * height;
        let rounds = log_2(smaller_extent) - 1;
        let mut frequencies = [0; 256];
        let mut min_brightness = 0;
        let mut max_brightness = 255;
        let bright_rounds = rounds - 2;
        let dark_rounds = rounds;
        for round in 0..rounds {
            let offset = 2u32.pow(round);
            let brighter = &mut |a: u8, b: u8| -> u8 { max_brightness.min(a.max(b)) };
            if round == 0 {
                extreme_around(&mut color_range, offset, &mut |a, b| {
                    frequencies[a as usize] += 1;
                    brighter(a, b)
                });
                let extremes = get_min_max_brightness(frequencies, num_pixels);
                min_brightness = extremes.0;
                max_brightness = extremes.1;
                extreme_around(&mut color_range, offset, brighter);
            } else if round < bright_rounds {
                extreme_around(&mut color_range, offset, brighter);
            }
            save_debug_image(
                &color_range,
                format!("brightest.{}.png", offset),
                args.debug,
            );
            if round < dark_rounds {
                extreme_around(&mut darkest, offset, &mut |a: u8, b: u8| -> u8 {
                    min_brightness.max(a.min(b))
                })
            }
            save_debug_image(&darkest, format!("darkest.{}.png", offset), args.debug);
        }
        stretch(&mut darkest, 2u32.pow(dark_rounds - 1));
        save_debug_image(
            &darkest,
            format!("darkest.stretched.{}.png", 2u32.pow(rounds - 1)),
            args.debug,
        );
        stretch(&mut color_range, 2u32.pow(bright_rounds - 1));
        save_debug_image(
            &color_range,
            format!("brightest.stretched.{}.png", 2u32.pow(rounds - 1)),
            args.debug,
        );
        subtract(&mut color_range, &darkest);
        save_debug_image(&color_range, "color_range.png".to_string(), args.debug);
        subtract(&mut img, &darkest);
        save_debug_image(&img, "img.unequalized.png".to_string(), args.debug);
        equalize(&mut img, color_range);
        img.save(cli::get_out_fname(&f)).unwrap();
    }
}
