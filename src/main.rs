use image::{open, GenericImageView, GrayImage};
use std::cmp::min;

const fn num_bits<T>() -> usize {
    std::mem::size_of::<T>() * 8
}

fn log_2(x: u32) -> u32 {
    assert!(x > 0);
    num_bits::<u32>() as u32 - x.leading_zeros() - 1
}

fn brightest(img1: &GrayImage, img2: &GrayImage) -> GrayImage {
    GrayImage::from_fn(img1.width(), img1.height(), |x, y| {
        let p1 = img1.get_pixel(x, y);
        let p2 = img2.get_pixel(x, y);
        if p1.0[0] < p2.0[0] {
            *p2
        } else {
            *p1
        }
    })
}

fn main() {
    let mut img = open("/tmp/paper.jpg").unwrap().grayscale().to_luma();
    let smaller_extent = min(img.width(), img.height());
    let rounds = log_2(smaller_extent) - 1;
    for round in 0..rounds {
        let offset = 2u32.pow(round);
        let win_width = img.width() - offset;
        let win_height = img.height() - offset;
        let orig = img.view(0, 0, win_width, win_height).to_image();
        let left = img.view(offset, 0, win_width, win_height).to_image();
        let up = img.view(0, offset, win_width, win_height).to_image();
        let diag = img.view(offset, offset, win_width, win_height).to_image();
        img = brightest(&brightest(&orig, &left), &brightest(&up, &diag));
    }
    img.save("/tmp/paper.png").unwrap();
}
