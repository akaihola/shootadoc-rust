use image::math::Rect;
use image::{open, GenericImage, GrayImage, Luma, Pixel};
use std::cmp::min;

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
    F: Fn(Luma<u8>, Luma<u8>) -> Luma<u8>,
{
    for (x, y, p) in img1.enumerate_pixels_mut() {
        *p = func(*p, *img2.get_pixel(x, y))
    }
}

fn apply_with_offset<F>(img: &mut GrayImage, dx: u32, dy: u32, func: F)
where
    F: Fn(Luma<u8>, Luma<u8>) -> Luma<u8>,
{
    let (width, height) = img.dimensions();
    for y in 0..height - dy {
        for x in 0..width - dx {
            img.put_pixel(
                x,
                y,
                func(*img.get_pixel(x, y), *img.get_pixel(x + dx, y + dy)),
            )
        }
    }
}

fn extreme<F>(img: &mut GrayImage, dx: u32, dy: u32, compare: F)
where
    F: Fn(u8, u8) -> bool,
{
    apply_with_offset(img, dx, dy, |p1, p2| {
        match compare(p1[0], p2[0]) {
            true => p1,
            false => p2,
        }
    })
}

fn extreme_around<F>(img: &mut GrayImage, offset: u32, compare: &F)
where
    F: Fn(u8, u8) -> bool,
{
    extreme(img, offset, 0, compare);
    extreme(img, 0, offset, compare);
}

fn pixel_difference(pixel1: Luma<u8>, pixel2: Luma<u8>) -> Luma<u8> {
    let mut result = pixel1.clone();
    result.apply2(&pixel2, &|a: u8, b: u8| a.saturating_sub(b));
    result
}

fn subtract(img1: &mut GrayImage, img2: &GrayImage) {
    apply2(img1, img2, pixel_difference)
}

fn stretch(img: &mut GrayImage, border: u32) {
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
    apply2(img, &color_range, |img_pixel, range_pixel| {
        let range_value = range_pixel[0] as f32;
        img_pixel
            .map_without_alpha(|value: u8| 255f32.min(value as f32 / range_value * 255f32) as u8)
    })
}

fn save_debug_image(img: &GrayImage, name: String, debug_mode: bool) -> () {
    if debug_mode {
        img.save(format!("/tmp/{}", name)).unwrap();
    }
}

fn main() {
    let brighter = |a, b| a > b;
    let darker = |a, b| a < b;
    let args = cli::parse_args();
    for f in args.in_file_path {
        let mut img: GrayImage = open(&f).unwrap().grayscale().to_luma();
        save_debug_image(&img, format!("darkest.0.png"), args.debug);
        let mut color_range = img.clone();
        let mut darkest = color_range.clone();
        let smaller_extent = min(img.width(), img.height());
        let rounds = log_2(smaller_extent) - 1;
        let border = 2u32.pow(rounds - 1);
        for round in 0..rounds {
            let offset = 2u32.pow(round);
            extreme_around(&mut color_range, offset, &brighter);
            save_debug_image(
                &color_range,
                format!("brightest.{}.png", offset),
                args.debug,
            );
            extreme_around(&mut darkest, offset, &darker);
            save_debug_image(&darkest, format!("darkest.{}.png", offset), args.debug);
        }
        stretch(&mut darkest, border);
        save_debug_image(
            &darkest,
            format!("darkest.stretched.{}.png", 2u32.pow(rounds - 1)),
            args.debug,
        );
        stretch(&mut color_range, border);
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
