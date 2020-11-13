use image::{open, GenericImageView, GrayImage};

fn brightest(img1: &GrayImage, img2: &GrayImage) -> GrayImage {
    GrayImage::from_fn(img1.width(), img1.height(), |x, y| {
        let p1 = img1.get_pixel(x, y);
        let p2 = img2.get_pixel(x, y);
        if p1.0[0] < p2.0[0] {
            *p2
        } else { *p1 }
    } )

}

fn main() {
    let img = open("/tmp/paper.jpg").unwrap().grayscale().to_luma();
    let offset = 50;
    let win_width = img.width() - offset;
    let win_height = img.height() - offset;
    let orig = img.view(0, 0, win_width, win_height);
    let left = img.view(offset, 0, win_width, win_height);
    let up = img.view(offset, 0, win_width, win_height);
    let b = brightest(
        &brightest(&orig.to_image(), &left.to_image()),
        &brightest(&orig.to_image(), &up.to_image()),
    );
    b.save("/tmp/paper.png").unwrap();
}
