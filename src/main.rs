use image::math::Rect;
use image::{open, GenericImage, GenericImageView, GrayImage, ImageBuffer, Pixel, Primitive};
use std::cmp::min;

mod cli;

const fn num_bits<T>() -> usize {
    std::mem::size_of::<T>() * 8
}

fn log_2(x: u32) -> u32 {
    assert!(x > 0);
    num_bits::<u32>() as u32 - x.leading_zeros() - 1
}

fn apply2<I, P, S, F>(img1: &mut ImageBuffer<P, Vec<S>>, img2: &I, func: F)
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
    F: Fn(P, P) -> P,
{
    for (x, y, p) in img1.enumerate_pixels_mut() {
        *p = func(*p, img2.get_pixel(x, y))
    }
}

fn apply_with_offset<P, S, F>(img: &mut ImageBuffer<P, Vec<S>>, offset: u32, func: F)
where
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
    F: Fn(P, P, P, P) -> P,
{
    let (width, height) = img.dimensions();
    for y in 0..height - offset {
        for x in 0..width - offset {
            img.put_pixel(
                x,
                y,
                func(
                    *img.get_pixel(x, y),
                    *img.get_pixel(x + offset, y),
                    *img.get_pixel(x, y + offset),
                    *img.get_pixel(x + offset, y + offset),
                ),
            )
        }
    }
}

fn extreme_around<P, S>(img: &mut ImageBuffer<P, Vec<S>>, offset: u32, pick_nth: usize)
where
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    apply_with_offset(img, offset, |p1, p2, p3, p4| {
        let mut pixels = vec![p1, p2, p3, p4];
        pixels.sort_by(|a, b| a.to_luma()[0].partial_cmp(&b.to_luma()[0]).unwrap());
        pixels[pick_nth]
    })
}

fn pixel_difference<P, S>(pixel1: P, pixel2: P) -> P
where
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    let mut result = pixel1.clone();
    result.apply2(&pixel2, &|a, b| match b > a {
        true => a - a,
        false => a - b,
    });
    result
}

fn subtract<I, P, S>(img1: &mut ImageBuffer<P, Vec<S>>, img2: &I)
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    apply2(img1, img2, pixel_difference)
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

fn equalize<I, P, S>(img: &mut ImageBuffer<P, Vec<S>>, color_range: I)
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    apply2(img, &color_range, |img_pixel, range_pixel| {
        let range_value = range_pixel.to_luma()[0].to_f32().unwrap();
        img_pixel.map_without_alpha(|value: S| {
            let value_f32 = value.to_f32().unwrap();
            let new_value_f32 = 255f32.min(value_f32 / range_value * 255f32);
            S::from(new_value_f32).unwrap()
        })
    })
}

fn save_debug_image(img: &GrayImage, name: String, debug_mode: bool) -> () {
    if debug_mode {
        img.save(format!("/tmp/{}", name)).unwrap();
    }
}

fn main() {
    let brighter = 2; // 3rd (the second brightest) out of 2x2 brightness-sorted pixels
    let darker = 0; // 1st (the darkest) of brightness-sorted 2x2 pixels
    let args = cli::parse_args();
    for f in args.in_file_path {
        let mut img = open(&f).unwrap().grayscale().to_luma();
        save_debug_image(&img, format!("darkest.0.png"), args.debug);
        let mut color_range = img.clone();
        let mut darkest = color_range.clone();
        let smaller_extent = min(img.width(), img.height());
        let rounds = log_2(smaller_extent) - 1;
        let border = 2u32.pow(rounds - 1);
        for round in 0..rounds {
            let offset = 2u32.pow(round);
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
