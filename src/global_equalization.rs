use image::GrayImage;

pub fn get_distribution(img: &GrayImage) -> [u32; 256] {
    let mut result = [0; 256];
    for p in img.pixels() {
        result[p[0] as usize] += 1;
    }
    for i in 0..256 {
        result[i] = ((result[i] as f32) + 1f32).log(1.1) as u32
    }
    result
}

fn smooth_and_get_distribution_local_extrema(
    distribution: &mut [u32; 256],
    right_to_left: bool,
) -> Vec<u8> {
    let mut prev_derivative = 0;
    let mut local_extrema = vec![];
    for i in 0..255 {
        let (prev_index, index, next_index) = if right_to_left {
            (256 - i, 255 - i, 254 - i)
        } else {
            (i - 1, i, i + 1)
        };
        distribution[index] = (distribution[index] + distribution[next_index]) / 2;
        if i > 0 {
            let derivative =
                i64::signum(distribution[index] as i64 - distribution[prev_index] as i64);
            if derivative != 0 && derivative != prev_derivative {
                if prev_derivative != 0 {
                    local_extrema.push(index as u8);
                }
                prev_derivative = derivative
            }
        }
    }
    if right_to_left {
        local_extrema.reverse()
    }
    local_extrema
}

pub fn get_distribution_local_extrema(distribution: &mut [u32; 256], debug_mode: bool) -> (u8, u8) {
    let mut round = 0;
    let local_extrema = loop {
        let local_extrema = smooth_and_get_distribution_local_extrema(distribution, round % 2 == 1);
        if debug_mode {
            println!("Turns after smoothing: {:?}", local_extrema)
        }
        round += 1;
        if local_extrema.len() <= 3 || round > 100 {
            break local_extrema;
        }
    };
    if local_extrema.len() < 2 {
        (0, 255)
    } else {
        let dark = 2 * local_extrema[0];
        let light = 255 - 2 * (255 - local_extrema[local_extrema.len() - 1]);
        if dark > light {
            (0, 255)
        } else {
            (dark, light)
        }
    }
}
