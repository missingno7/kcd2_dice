use std::cmp::max;
use image::{GrayImage, Luma};
use std::path::Path;
use crate::Die;
use image::RgbImage;
use image::Rgb;

pub fn is_bust(faces: &[usize; 6]) -> bool {
    let mut counts = [0u8; 6];

    for &f in faces {
        counts[f] += 1;
    }

    // 1 or 5 score
    if counts[0] > 0 || counts[4] > 0 {
        return false;
    }

    // triples or more score
    for c in counts {
        if c >= 3 {
            return false;
        }
    }

    // check straights
    let straight_1_6 = counts.iter().all(|&c| c == 1);
    if straight_1_6 {
        return false;
    }

    let straight_1_5 =
        counts[0] == 1 &&
            counts[1] == 1 &&
            counts[2] == 1 &&
            counts[3] == 1 &&
            counts[4] == 1;

    if straight_1_5 {
        return false;
    }

    let straight_2_6 =
        counts[1] == 1 &&
            counts[2] == 1 &&
            counts[3] == 1 &&
            counts[4] == 1 &&
            counts[5] == 1;

    if straight_2_6 {
        return false;
    }

    true
}

/// Decode n in [0, 215] into 3 base-6 digits (each in 0..5).
#[inline]
fn decode_base6_3(mut n: u32) -> [usize; 3] {
    let a = (n % 6) as usize;
    n /= 6;
    let b = (n % 6) as usize;
    n /= 6;
    let c = (n % 6) as usize;
    [a, b, c]
}

/// Expand (dice_types, counts) into exactly 6 concrete dice.
/// Panics if counts don't sum to 6.
pub fn expand_to_six(dice_types: &[Die], counts: &[u8]) -> [Die; 6] {
    assert_eq!(
        dice_types.len(),
        counts.len(),
        "dice_types and counts must have same length"
    );

    let total: usize = counts.iter().map(|&c| c as usize).sum();
    assert_eq!(total, 6, "counts must sum to 6 dice");

    let mut out: Vec<Die> = Vec::with_capacity(6);
    for (d, &c) in dice_types.iter().zip(counts.iter()) {
        for _ in 0..c {
            out.push(d.clone());
        }
    }

    let mut dice = Vec::with_capacity(6);

    for (d, &c) in dice_types.iter().zip(counts.iter()) {
        for _ in 0..c {
            dice.push(d.clone());
        }
    }

    assert_eq!(dice.len(), 6);

    [
        dice[0].clone(),
        dice[1].clone(),
        dice[2].clone(),
        dice[3].clone(),
        dice[4].clone(),
        dice[5].clone(),
    ]
}

/// faces are 0..5 (0=1, 4=5)
pub fn color_for_faces(faces: &[usize; 6], brightness: u8) -> Rgb<u8> {
    // Count faces
    let mut counts = [0u8; 6];
    for &f in faces {
        debug_assert!(f < 6);
        counts[f] += 1;
    }

    // Bust rule (your earlier definition incl. straights)
    if is_bust(faces) {
        return Rgb([brightness, 0, 0]);
    }

    let c1 = counts[0] as f32; // ones
    let c5 = counts[4] as f32; // fives

    // Max-of-a-kind
    let max_kind = *counts.iter().max().unwrap_or(&0) as f32; // 0..6

    // Straights
    let straight_1_6 = counts.iter().all(|&c| c == 1);
    let straight_1_5 =
        counts[0] == 1 && counts[1] == 1 && counts[2] == 1 && counts[3] == 1 && counts[4] == 1;
    let straight_2_6 =
        counts[1] == 1 && counts[2] == 1 && counts[3] == 1 && counts[4] == 1 && counts[5] == 1;
    let is_straight = straight_1_6 || straight_1_5 || straight_2_6;

    if is_straight {
        return Rgb([brightness, brightness, 0]);
    }

    // Normalize helpers (0..1)
    let br = brightness as f32 / 255.0;

    // Ones/Fives contribution (scale by count)
    let ones_w = (c1 / 6.0).clamp(0.0, 1.0);
    let fives_w = (c5 / 6.0).clamp(0.0, 1.0);

    // Triples+ contribution:
    // 0 if <3, then ramps up to 1.0 at 6-of-kind
    let kind_w = if max_kind < 2.5 {
        0.0
    } else {
        ((max_kind - 2.0) / 4.0).clamp(0.0, 1.0) // 3->0.25, 6->1
    };

    //println!("{}, {}", ones_w, fives_w);

    // Base grayscale
    let mut r = 0.2;
    let mut g = 0.2;
    let mut b = 0.2;

    // Mix rules:
    // - Ones push green
    g = (g + 0.9 * ones_w);

    // - Fives push blue
    b = (b + 0.9 * fives_w);

    // - Triples+ red tint (R)
    r = (r + 0.3 * kind_w);

    //println!("{}, {} {}", r, g, b);

   // let faces_plus_one = faces.map(|f| f + 1);
   //  if r==g && g==b{
   //      println!("{} {} {}, {:?} - {:?} {:?} {:?} {:?}",r,g,b,faces_plus_one, ones_w, fives_w, kind_w, max_kind);
   //  }

     let max = r.max(g).max(b);

    (r,g,b) = if max > 0.0 {
        (r / max, g / max, b / max)
    } else {
        (0.0, 0.0, 0.0)
    };

    //println!("{}, {} {}", r, g, b);

    Rgb([
        (r*br * 255.0).round() as u8,
        (g*br * 255.0).round() as u8,
        (b*br * 255.0).round() as u8,
    ])
}

/// Renders a 216x216 image where each pixel corresponds to a 6-dice outcome.
/// Brightness is proportional to probability of that exact outcome (product of per-die face probs),
/// normalized by the maximum probability across all outcomes.
///
/// Coloring:
/// - contains at least one 1  -> green-ish
/// - contains at least one 5  -> blue-ish
/// - contains both 1 and 5    -> yellow-ish
/// - contains neither         -> grayscale
///
/// Mapping:
/// - x encodes dice 0,1,2 (base-6 index 0..215)
/// - y encodes dice 3,4,5 (base-6 index 0..215)
///
/// `gamma` can enhance contrast:
/// - 1.0 => linear
/// - <1.0 => brightens low probabilities (e.g. 0.4..0.7)
pub fn render_probability_image_216_from_counts(
    dice_types: &[Die],
    counts: &[u8],
    out_path: impl AsRef<Path>,
    gamma: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let dice: [Die; 6] = expand_to_six(dice_types, counts);

    let width: u32 = 216;
    let height: u32 = 216;

    // First pass: find max probability for normalization
    let mut max_p = 0.0_f64;
    for y in 0..height {
        let y_faces = decode_base6_3(y);
        for x in 0..width {
            let x_faces = decode_base6_3(x);

            let p =
                dice[0].probs[x_faces[0]] *
                    dice[1].probs[x_faces[1]] *
                    dice[2].probs[x_faces[2]] *
                    dice[3].probs[y_faces[0]] *
                    dice[4].probs[y_faces[1]] *
                    dice[5].probs[y_faces[2]];

            if p > max_p {
                max_p = p;
            }
        }
    }

    if max_p <= 0.0 {
        return Err("max probability is zero (check dice probabilities)".into());
    }

    // Second pass: render pixels (RGB)
    let mut img = RgbImage::new(width, height);
    let gamma = if gamma.is_finite() && gamma > 0.0 { gamma } else { 1.0 };

    for y in 0..height {
        let y_faces = decode_base6_3(y);
        for x in 0..width {
            let x_faces = decode_base6_3(x);

            let faces = [
                x_faces[0],
                x_faces[1],
                x_faces[2],
                y_faces[0],
                y_faces[1],
                y_faces[2],
            ];

            let p =
                dice[0].probs[faces[0]] *
                    dice[1].probs[faces[1]] *
                    dice[2].probs[faces[2]] *
                    dice[3].probs[faces[3]] *
                    dice[4].probs[faces[4]] *
                    dice[5].probs[faces[5]];

            let mut v = (p / max_p).clamp(0.0, 1.0);
            v = v.powf(gamma);
            let b = (v * 255.0).round() as u8;

let px = color_for_faces(&faces, b);
img.put_pixel(x, y, px);
        }
    }

    img.save(out_path)?;
    Ok(())
}