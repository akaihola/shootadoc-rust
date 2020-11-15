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

fn brightest<I, P, S>(img1: &I, img2: &I) -> ImageBuffer<P, Vec<S>>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel<Subpixel = S> + 'static,
    S: Primitive + 'static,
{
    ImageBuffer::from_fn(img1.width(), img1.height(), |x, y| {
        let p1 = img1.get_pixel(x, y);
        let p2 = img2.get_pixel(x, y);
        if p1.to_luma()[0] < p2.to_luma()[0] {
            p2
        } else {
            p1
        }
    })
}

fn main() {
    let args = cli::parse_args();
    for f in args.in_file_path {
        let mut img: image::GrayImage = open(&f).unwrap().grayscale().to_luma();
        let smaller_extent = min(img.width(), img.height());
        let rounds = log_2(smaller_extent) - 1;
        for round in 0..rounds {
            let offset = 2u32.pow(round);
            let win_width = img.width() - offset;
            let win_height = img.height() - offset;
            let orig = img.view(0, 0, win_width, win_height);
            let left = img.view(offset, 0, win_width, win_height);
            let up = img.view(0, offset, win_width, win_height);
            let diag = img.view(offset, offset, win_width, win_height);
            let top_pixels = brightest(&orig, &left);
            let bottom_pixels = brightest(&up, &diag);
            img = brightest(&top_pixels, &bottom_pixels);
        }
        img.save(cli::get_out_fname(&f)).unwrap();
    }
}
