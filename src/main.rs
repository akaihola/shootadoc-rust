use image::{open, GenericImageView, ImageBuffer, Pixel, Primitive};
use std::cmp::min;

mod cli;

const fn num_bits<T>() -> usize {
    std::mem::size_of::<T>() * 8
}

fn log_2(x: u32) -> u32 {
    assert!(x > 0);
    num_bits::<u32>() as u32 - x.leading_zeros() - 1
}

fn apply2<I, P, S, F>(img1: I, img2: I, func: F) -> ImageBuffer<P, Vec<S>>
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

fn extreme<I, P, S, F>(img1: I, img2: I, compare: F) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
    F: Fn(S, S) -> bool,
{
    apply2(img1, img2, |p1, p2| {
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

fn difference<I, P, S>(img1: I, img2: I) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    apply2(img1, img2, pixel_difference)
}

fn main() {
    let brighter = |a, b| a > b;
    let darker = |a, b| a < b;
    let args = cli::parse_args();
    for f in args.in_file_path {
        let mut brightest = open(&f).unwrap().grayscale().to_luma();
        let mut darkest = brightest.clone();
        let smaller_extent = min(brightest.width(), brightest.height());
        let rounds = log_2(smaller_extent) - 1;
        for round in 0..rounds {
            let offset = 2u32.pow(round);
            brightest = extreme_around(brightest, offset, &brighter);
            darkest = extreme_around(darkest, offset, &darker);
        }
        difference(brightest, darkest)
            .save(cli::get_out_fname(&f))
            .unwrap();
    }
}
