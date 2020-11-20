use image::math::Rect;
use image::{open, GenericImage, GrayImage, Luma, Pixel};
use std::cmp::min;

mod cli;

fn apply2<F>(img1: &mut GrayImage, img2: &GrayImage, func: F)
where
    F: Fn(Luma<u8>, Luma<u8>) -> Luma<u8>,
{
    for (x, y, p) in img1.enumerate_pixels_mut() {
        *p = func(*p, *img2.get_pixel(x, y))
    }
}

fn apply_with_offset<F>(img: &mut GrayImage, offset: u32, func: F)
where
    F: Fn(
        Luma<u8>,
        Luma<u8>,
        Luma<u8>,
        Luma<u8>,
        Luma<u8>,
        Luma<u8>,
        Luma<u8>,
        Luma<u8>,
        Luma<u8>,
    ) -> Luma<u8>,
{
    let (width, height) = img.dimensions();
    for y in 0..height - offset {
        for x in 0..width - offset {
            img.put_pixel(
                x,
                y,
                func(
                    *img.get_pixel(x, y),
                    *img.get_pixel(x + offset / 3, y),
                    *img.get_pixel(x + 2 * offset / 3, y),
                    *img.get_pixel(x, y + offset / 3),
                    *img.get_pixel(x + offset / 3, y + offset / 3),
                    *img.get_pixel(x + 2 * offset / 3, y + offset / 3),
                    *img.get_pixel(x, y + 2 * offset / 3),
                    *img.get_pixel(x + offset / 3, y + 2 * offset / 3),
                    *img.get_pixel(x + 2 * offset / 3, y + 2 * offset / 3),
                ),
            )
        }
    }
}

fn extreme_around(img: &mut GrayImage, offset: u32, pick_nth: usize) {
    apply_with_offset(
        img,
        offset,
        |p11: Luma<u8>,
         p12: Luma<u8>,
         p13: Luma<u8>,
         p21: Luma<u8>,
         p22: Luma<u8>,
         p23: Luma<u8>,
         p31: Luma<u8>,
         p32: Luma<u8>,
         p33: Luma<u8>| {
            let mut pixels = vec![p11, p12, p13, p21, p22, p23, p31, p32, p33];
            pixels.sort_by(|a, b| a[0].partial_cmp(&b[0]).unwrap());
            pixels[pick_nth]
        },
    )
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
        img_pixel.map_without_alpha(|value: u8| (value as f32 / range_value * 255f32) as u8)
    })
}

fn save_debug_image(img: &GrayImage, name: String, debug_mode: bool) -> () {
    if debug_mode {
        img.save(format!("/tmp/{}", name)).unwrap();
    }
}

fn main() {
    let brighter = 6;
    let darker = 0;
    let args = cli::parse_args();
    for f in args.in_file_path {
        let mut img: GrayImage = open(&f).unwrap().grayscale().to_luma();
        save_debug_image(&img, format!("darkest.0.png"), args.debug);
        let mut color_range = img.clone();
        let mut darkest = color_range.clone();
        let smaller_extent = min(img.width(), img.height());
        let rounds = (smaller_extent as f32).log(3.0) as u32;
        let border = 3u32.pow(rounds - 1);
        for round in 0..rounds {
            let offset = 3u32.pow(round);
            extreme_around(&mut color_range, offset, brighter);
            save_debug_image(
                &color_range,
                format!("brightest.{}.png", offset),
                args.debug,
            );
            extreme_around(&mut darkest, offset, darker);
            save_debug_image(&darkest, format!("darkest.{}.png", offset), args.debug);
        }
        stretch(&mut darkest, border);
        save_debug_image(
            &darkest,
            format!("darkest.stretched.{}.png", border),
            args.debug,
        );
        stretch(&mut color_range, border);
        save_debug_image(
            &color_range,
            format!("brightest.stretched.{}.png", border),
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
