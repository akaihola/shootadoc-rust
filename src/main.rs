use image::math::Rect;
use image::{open, GenericImage, GrayImage, ImageBuffer, Luma, Pixel, Primitive};

mod cli;
mod global_equalization;
mod imageops;
mod math;
mod pixelops;

fn replace_with_surrounding_extreme<F>(img: &mut GrayImage, distance: u32, compare: &F)
where
    F: Fn(u8, u8) -> u8,
{
    imageops::shift_and_apply2(img, distance, 0, compare);
    imageops::shift_and_apply2(img, 0, distance, compare);
}

fn center_and_stretch<P, S>(img: &mut ImageBuffer<P, Vec<S>>, border: u32)
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

fn local_equalize(
    img: &mut GrayImage,
    local_black: GrayImage,
    local_range: GrayImage,
    debug_mode: bool,
) {
    save_debug_image(&img, "corrected.unequalized.png".to_string(), debug_mode);
    imageops::apply3(
        img,
        &local_black,
        &local_range,
        |img_pixel, black, range| pixelops::equalize(img_pixel, black, range),
    )
}

fn save_debug_image(img: &GrayImage, name: String, debug_mode: bool) -> () {
    if debug_mode {
        img.save(format!("/tmp/{}", name)).unwrap();
    }
}

fn main() {
    let args = cli::parse_args();
    for f in args.in_file_path {
        let original_image = open(&f).unwrap().grayscale().to_luma();
        save_debug_image(&original_image, format!("darkest.0.png"), args.debug);
        let (width, height) = original_image.dimensions();
        let smaller_extent = width.min(height);
        let max_rounds = math::log_2(smaller_extent) - 1;
        let white_rounds = max_rounds - 2;
        let black_rounds = max_rounds;
        let mut local_black = original_image.clone();
        replace_with_surrounding_extreme(&mut local_black, 1, &pixelops::darker);
        save_debug_image(&local_black, format!("darkest.0.png"), args.debug);
        let mut local_white = local_black.clone();
        for round in 1..white_rounds.max(black_rounds) {
            let distance = 2u32.pow(round);
            if round < white_rounds {
                replace_with_surrounding_extreme(&mut local_white, distance, &pixelops::brighter);
            }
            save_debug_image(
                &local_white,
                format!("brightest.{}.png", distance),
                args.debug,
            );
            if round < black_rounds {
                replace_with_surrounding_extreme(&mut local_black, distance, &pixelops::darker)
            }
            save_debug_image(
                &local_black,
                format!("darkest.{}.png", distance),
                args.debug,
            );
        }
        let black_center = 2u32.pow(black_rounds - 1);
        center_and_stretch(&mut local_black, black_center);
        save_debug_image(
            &local_black,
            format!("darkest.stretched.{}.png", black_center),
            args.debug,
        );
        let white_center = 2u32.pow(white_rounds - 1);
        center_and_stretch(&mut local_white, white_center);
        save_debug_image(
            &local_white,
            format!("brightest.stretched.{}.png", white_center),
            args.debug,
        );

        let mut local_range = local_white;
        imageops::subtract(&mut local_range, &local_black);
        save_debug_image(&local_range, "color_range.png".to_string(), args.debug);

        let mut corrected_image = original_image;
        local_equalize(&mut corrected_image, local_black, local_range, args.debug);
        let mut distribution = global_equalization::get_distribution(&corrected_image);
        let (global_black, global_white) =
            global_equalization::get_distribution_local_extrema(&mut distribution, args.debug);
        if global_white > global_black && global_black > 0 && global_white < 255 {
            let global_range = global_white - global_black;
            if args.debug {
                println!(
                    "{}..{} range is {}",
                    global_black, global_white, global_range
                )
            }
            for p in corrected_image.pixels_mut() {
                *p = Luma([pixelops::equalize(p[0], global_black, global_range)])
            }
        }
        corrected_image.save(cli::get_out_fname(&f)).unwrap();
    }
}
