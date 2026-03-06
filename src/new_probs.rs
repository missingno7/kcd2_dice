use std::collections::{BTreeMap, HashMap};
use crate::render::expand_to_six;
use crate::Die;

pub fn calc_score(counts: &[usize; 6]) -> u32 {
    // -------- Straights --------

    if counts.iter().all(|&c| c == 1) {
        return 1500; // 1-6
    }

    let is_straight_1_5 =
        counts[0] >= 1 && counts[1] >= 1 && counts[2] >= 1 && counts[3] >= 1 && counts[4] >= 1;

    if is_straight_1_5 {
        let extra_ones = counts[0].saturating_sub(1) as u32;
        let extra_fives = counts[4].saturating_sub(1) as u32;
        return 500 + extra_ones * 100 + extra_fives * 50;
    }

    let is_straight_2_6 =
        counts[1] >= 1 && counts[2] >= 1 && counts[3] >= 1 && counts[4] >= 1 && counts[5] >= 1;

    if is_straight_2_6 {
        let extra_ones = counts[0] as u32;
        let extra_fives = counts[4].saturating_sub(1) as u32;
        return 750 + extra_ones * 100 + extra_fives * 50;
    }

    // -------- Multiples + singles --------

    let mut score: u32 = 0;

    for face in 0..6 {
        let c = counts[face];

        if c >= 3 {
            let base = if face == 0 {
                1000
            } else if face == 4 {
                500
            } else {
                (face as u32 + 1) * 100
            };

            let mult = match c {
                3 => 1,
                4 => 2,
                5 => 4,
                6 => 8,
                _ => unreachable!(),
            };

            score += base * mult;
        } else {
            // Singles only if there is no 3+ of that face
            if face == 0 {
                score += c as u32 * 100;
            } else if face == 4 {
                score += c as u32 * 50;
            }
        }
    }

    score
}


/// Encodes counts[0..6] (each 0..=6) into a unique integer key.
/// Base-7 is enough because each count is in 0..=6.
#[inline]
fn encode_counts(c: &[u8; 6]) -> u32 {
    let mut key: u32 = 0;
    let mut mul: u32 = 1;
    for i in 0..6 {
        key += (c[i] as u32) * mul;
        mul *= 7;
    }
    key
}

/// Returns probabilities for unique outcomes represented as face-count histograms.
/// Output: Vec of (counts_per_face, probability), where counts_per_face[0] is # of 1s, etc.
///
/// This is order-independent (multiset), so there are at most 462 entries.
pub fn enumerate_histogram_probabilities(
    dice_types: &[Die],
    counts_by_type: &[u8],
) -> Vec<([u8; 6], f64)> {
    // Expand into exactly 6 dice
    let dice: [Die; 6] = expand_to_six(dice_types, counts_by_type);

    // DP map: encoded_counts -> (counts, prob)
    // We store counts separately so we don't need to decode.
    let mut dp: HashMap<u32, ([u8; 6], f64)> = HashMap::new();
    let start = [0u8; 6];
    dp.insert(encode_counts(&start), (start, 1.0));

    for die in dice.iter() {
        let mut next: HashMap<u32, ([u8; 6], f64)> = HashMap::new();

        for (_k, (state_counts, state_p)) in dp.iter() {
            if *state_p == 0.0 {
                continue;
            }

            for face in 0..6 {
                let p_face = die.probs[face];
                if p_face == 0.0 {
                    continue;
                }

                let mut nc = *state_counts;
                nc[face] += 1;

                let key = encode_counts(&nc);
                let entry = next.entry(key).or_insert((nc, 0.0));
                entry.1 += state_p * p_face;
            }
        }

        dp = next;
    }

    let mut out: Vec<([u8; 6], f64)> = dp.into_values().collect();

    // Optional: sort by probability descending
    out.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    out
}

pub fn print_histogram_ev(hist: &[([u8; 6], f64)]) {
    let mut rows: Vec<([u8; 6], f64, u32, f64)> = Vec::new();

    for (counts, p) in hist {
        let counts_usize: [usize; 6] = [
            counts[0] as usize,
            counts[1] as usize,
            counts[2] as usize,
            counts[3] as usize,
            counts[4] as usize,
            counts[5] as usize,
        ];

        let score = calc_score(&counts_usize);
        let ev = *p * score as f64;

        rows.push((*counts, *p, score, ev));
    }

    // sort by EV descending
    rows.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap());

    let mut total_ev = 0.0;

    for (counts, p, score, ev) in &rows {
        // println!(
        //     "{}  p={:.4}%  score={}  ev={:.3}",
        //     format_faces(counts),
        //     p * 100.0,
        //     score,
        //     ev
        // );

        println!(
            "{}\t{}",
            p,
            score,
        );

        total_ev += ev;
    }

    println!("--------------------------------");
    println!("Total EV = {:.3}", total_ev);
}

pub fn format_faces(counts: &[u8; 6]) -> String {
    let mut parts = Vec::new();

    for face in 0..6 {
        let c = counts[face];
        if c > 0 {
            parts.push(format!("{}x{}", c, face + 1));
        }
    }

    parts.join(" ")
}

pub fn evaluate_histogram(dice_types: &[Die], counts_by_type: &[u8]) {
    assert_eq!(
        dice_types.len(),
        counts_by_type.len(),
        "dice_types and counts must match"
    );

    let total: usize = counts_by_type.iter().map(|&c| c as usize).sum();
    assert_eq!(total, 6, "total dice must be 6");

    println!("=== Dice set ===");

    for (die, &count) in dice_types.iter().zip(counts_by_type.iter()) {
        println!("{}x {}", count, die.name);
    }

    println!("----------------------------");

    let hist = enumerate_histogram_probabilities(dice_types, counts_by_type);

    print_histogram_ev(&hist);
}

pub fn print_score_distribution(hist: &[([u8; 6], f64)]) {
    // score -> summed probability
    let mut by_score: BTreeMap<u32, f64> = BTreeMap::new();

    for (counts, p) in hist {
        let counts_usize: [usize; 6] = [
            counts[0] as usize,
            counts[1] as usize,
            counts[2] as usize,
            counts[3] as usize,
            counts[4] as usize,
            counts[5] as usize,
        ];

        let score = calc_score(&counts_usize);
        *by_score.entry(score).or_insert(0.0) += *p;
    }

    // (score, p, ev)
    let mut rows: Vec<(u32, f64, f64)> = by_score
        .iter()
        .map(|(&score, &p)| (score, p, p * score as f64))
        .collect();

    // sort by EV descending
    rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

    let mut total_ev = 0.0;
    let mut total_p = 0.0;

    for (score, p, ev) in &rows {

        println!(
            "p={:.4}%  score={}  ev={:.3}",
            p * 100.0,
            score,
            ev
        );

        //         println!(
        //     "{}\t{}\t{}",
        //     p,
        //     score,
        //     ev
        // );

        total_ev += *ev;
        total_p += *p;
    }

    println!("--------------------------------");
    println!("Total probability = {:.4}%", total_p * 100.0);
    println!("Total EV = {:.3}", total_ev);
}