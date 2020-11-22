use image::{open, GrayImage, Luma};
use std::cmp::min;

mod cli;

fn apply2<F>(img1: &mut GrayImage, img2: &GrayImage, func: F)
where
    F: Fn(u8, u8) -> u8,
{
    for (x, y, p) in img1.enumerate_pixels_mut() {
        *p = Luma([func(p[0], img2.get_pixel(x, y)[0])])
    }
}

fn apply_with_offset<F>(img: GrayImage, offset: u32, func: F) -> GrayImage
where
    F: Fn(Vec<u8>) -> u8,
{
    let (width, height) = img.dimensions();
    let mut result = GrayImage::new(width, height);
    let next = (offset as i32) / 3;
    for y in 0..height as i32 {
        for x in 0..width as i32 {
            let (x1, x2, x3) = (x - next, x, x + next);
            let (y1, y2, y3) = (y - next, y, y + next);
            let area = if y1 < 0 {
                if x1 < 0 {
                    vec![(x2, y2), (x3, y2), (x2, y3), (x3, y3)]
                } else if x3 < width as i32 {
                    vec![(x1, y2), (x2, y2), (x3, y2), (x1, y3), (x2, y3), (x3, y3)]
                } else {
                    vec![(x1, y2), (x2, y2), (x1, y3), (x2, y3)]
                }
            } else if y3 < height as i32 {
                if x1 < 0 {
                    vec![(x2, y1), (x3, y1), (x2, y2), (x3, y2), (x2, y3), (x3, y3)]
                } else if x3 < width as i32 {
                    vec![
                        (x1, y1),
                        (x2, y1),
                        (x3, y1),
                        (x1, y2),
                        (x2, y2),
                        (x3, y2),
                        (x1, y3),
                        (x2, y3),
                        (x3, y3),
                    ]
                } else {
                    vec![(x1, y1), (x2, y1), (x1, y2), (x2, y2), (x1, y3), (x2, y3)]
                }
            } else {
                if x1 < 0 {
                    vec![(x2, y1), (x3, y1), (x2, y2), (x3, y2)]
                } else if x3 < width as i32 {
                    vec![(x1, y1), (x2, y1), (x3, y1), (x1, y2), (x2, y2), (x3, y2)]
                } else {
                    vec![(x1, y1), (x2, y1), (x1, y2), (x2, y2)]
                }
            };
            let pixels: Vec<u8> = area
                .iter()
                .map(|(ax, ay)| img.get_pixel(*ax as u32, *ay as u32)[0])
                .collect();
            result.put_pixel(x as u32, y as u32, Luma([func(pixels)]))
        }
    }
    result
}

fn extreme_around(img: GrayImage, offset: u32, pick_nth: i8) -> GrayImage {
    apply_with_offset(img, offset, |pixels: Vec<u8>| {
        let mut sorted = pixels.clone();
        sorted.sort_unstable();
        let len = sorted.len();
        if pick_nth >= 0 {
            sorted[len * (pick_nth as usize) / 9]
        } else {
            sorted[len - len * (-pick_nth - 1) as usize / 9 - 1]
        }
    })
}

fn subtract(img1: &mut GrayImage, img2: &GrayImage) {
    apply2(img1, img2, |a, b| a.saturating_sub(b))
}

fn equalize(img: &mut GrayImage, color_range: GrayImage) {
    apply2(img, &color_range, |img_pixel, range_pixel| {
        let range_value = range_pixel as f32;
        (img_pixel as f32 / range_value * 255f32) as u8
    })
}

fn save_debug_image(img: &GrayImage, name: String, debug_mode: bool) -> () {
    if debug_mode {
        img.save(format!("/tmp/{}", name)).unwrap();
    }
}

fn main() {
    let brighter = -2;
    let darker = 0;
    let args = cli::parse_args();
    for f in args.in_file_path {
        let mut img: GrayImage = open(&f).unwrap().grayscale().to_luma();
        save_debug_image(&img, format!("darkest.0.png"), args.debug);
        let mut color_range = img.clone();
        let mut darkest = color_range.clone();
        let smaller_extent = min(img.width(), img.height());
        let rounds = (smaller_extent as f32).log(3.0) as u32;
        let border = 3u32.pow(rounds);
        for round in 0..rounds + 1 {
            let offset = 3u32.pow(round);
            color_range = extreme_around(color_range, offset, brighter);
            save_debug_image(
                &color_range,
                format!("brightest.{}.png", offset),
                args.debug,
            );
            darkest = extreme_around(darkest, offset, darker);
            save_debug_image(&darkest, format!("darkest.{}.png", offset), args.debug);
        }
        save_debug_image(
            &darkest,
            format!("darkest.stretched.{}.png", border),
            args.debug,
        );
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
