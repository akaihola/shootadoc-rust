use image::imageops::replace;
use image::math::Rect;
use image::{open, GenericImage, GenericImageView, ImageBuffer, Pixel, Primitive};
use std::cmp::min;

mod cli;

const fn num_bits<T>() -> usize {
    std::mem::size_of::<T>() * 8
}

fn log_2(x: u32) -> u32 {
    assert!(x > 0);
    num_bits::<u32>() as u32 - x.leading_zeros() - 1
}

fn map2<I, P, S, F>(img1: I, img2: &I, func: F) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
    F: Fn(P, P) -> P,
{
    ImageBuffer::from_fn(img1.width(), img1.height(), |x, y| {
        let p1: P = img1.get_pixel(x, y);
        let p2: P = img2.get_pixel(x, y);
        func(p1, p2)
    })
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

fn extreme<I, P, S, F>(img1: I, img2: I, compare: F) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
    F: Fn(S, S) -> bool,
{
    map2(img1, &img2, |p1, p2| {
        match compare(p1.to_luma()[0], p2.to_luma()[0]) {
            true => p1,
            false => p2,
        }
    })
}

fn extreme_around<I, P, S, F>(img: I, offset: u32, compare: &F) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
    F: Fn(S, S) -> bool,
{
    let win_width = img.width() - offset;
    let win_height = img.height() - offset;
    let orig = img.view(0, 0, win_width, win_height);
    let left = img.view(offset, 0, win_width, win_height);
    let up = img.view(0, offset, win_width, win_height);
    let diag = img.view(offset, offset, win_width, win_height);
    let top_pixels = extreme(orig, left, compare);
    let bottom_pixels = extreme(up, diag, compare);
    extreme(top_pixels, bottom_pixels, compare)
}

fn pixel_difference<P, S>(pixel1: P, pixel2: P) -> P
where
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    let mut result = pixel1.clone();
    result.apply2(&pixel2, &|a, b| a - b);
    result
}

fn difference<I, P, S>(img1: I, img2: &I) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    map2(img1, img2, pixel_difference)
}

fn stretch<I, P, S>(img: I, border: u32) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    let (w, h) = img.dimensions();
    let (rw, rh) = (w + 2 * border - 1, h + 2 * border - 1);
    let mut result = ImageBuffer::new(rw, rh);
    replace(&mut result, &img, border, border);
    let top = Rect {
        x: 0,
        y: border,
        width: rw,
        height: 1,
    };
    let bottom = Rect {
        x: 0,
        y: border + h - 1,
        width: rw,
        height: 1,
    };
    let left = Rect {
        x: border,
        y: 0,
        width: 1,
        height: rh,
    };
    let right = Rect {
        x: border + w - 1,
        y: 0,
        width: 1,
        height: rh,
    };
    for y in 0..border {
        result.copy_within(top, 0, y);
        result.copy_within(bottom, 0, rh - y - 1);
    }
    for x in 0..border {
        result.copy_within(left, x, 0);
        result.copy_within(right, rw - x - 1, 0);
    }
    result
}

fn equalize<I, P, S>(img: I, brightest: I, darkest: &I) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    let color_range = difference(brightest, darkest);
    let mut result = difference(img, darkest);
    apply2(&mut result, &color_range, |img_pixel, range_pixel| {
        let range_value = range_pixel.to_luma()[0].to_f32().unwrap();
        img_pixel.map_without_alpha(|value: S| {
            let value_f32 = value.to_f32().unwrap();
            let new_value_f32 = value_f32 / range_value * 255f32;
            S::from(new_value_f32).unwrap()
        })
    });
    result
}

fn main() {
    let brighter = |a, b| a > b;
    let darker = |a, b| a < b;
    let args = cli::parse_args();
    for f in args.in_file_path {
        let img = open(&f).unwrap().grayscale().to_luma();
        let mut brightest = img.clone();
        let mut darkest = brightest.clone();
        let smaller_extent = min(img.width(), img.height());
        let rounds = log_2(smaller_extent) - 1;
        for round in 0..rounds {
            let offset = 2u32.pow(round);
            brightest = extreme_around(brightest, offset, &brighter);
            darkest = extreme_around(darkest, offset, &darker);
        }
        let brightest_stretched = stretch(brightest, 2u32.pow(rounds - 1));
        let darkest_stretched = stretch(darkest, 2u32.pow(rounds - 1));
        equalize(img, brightest_stretched, &darkest_stretched)
            .save(cli::get_out_fname(&f))
            .unwrap();
    }
}
