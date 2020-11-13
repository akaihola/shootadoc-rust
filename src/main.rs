use image::open;

fn main() {
    let img = open("journal.jpg").unwrap().grayscale().to_luma();
    let result = imageproc::contrast::adaptive_threshold(&img, 150);
    result.save("out.jpg").unwrap();
}
