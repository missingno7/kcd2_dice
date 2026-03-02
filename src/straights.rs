use crate::Die;

/// Returns (p_1to5, p_2to6, p_1to6) for a single roll of 6 dice.
/// Uses DP over (mask of present numbers, joker_count).
///
/// Die model:
/// - if die.is_devil == false: face 0..5 => numbers 1..6
/// - if die.is_devil == true:  face 0 is Joker, faces 1..5 => numbers 2..6 (no "1" outcome)
pub fn probabilities_straights(dice: &[Die], counts: &[u8]) -> (f64, f64, f64) {
    assert_eq!(dice.len(), counts.len());
    let total: usize = counts.iter().map(|&c| c as usize).sum();
    assert_eq!(total, 6, "counts must sum to 6 dice");

    // dp[mask][j] stored in a flat array: index = mask*7 + j
    // mask: 0..64 (6 bits), j: 0..6
    let mut dp = [0.0f64; 64 * 7];
    dp[0 * 7 + 0] = 1.0;

    // Helper to set a bit for number n in 1..=6 -> bit (n-1)
    #[inline]
    fn bit_for_number(n: u8) -> u8 {
        1u8 << (n - 1)
    }

    for (die, &cnt) in dice.iter().zip(counts.iter()) {
        for _ in 0..cnt {
            let mut next = [0.0f64; 64 * 7];

            for mask in 0..64u8 {
                for j in 0..=6u8 {
                    let cur = dp[mask as usize * 7 + j as usize];
                    if cur == 0.0 {
                        continue;
                    }

                    if !die.is_devil {
                        // Normal die: faces 0..5 -> numbers 1..6
                        for face in 0..6 {
                            let p = die.probs[face];
                            if p == 0.0 {
                                continue;
                            }
                            let number = (face as u8) + 1; // 1..6
                            let new_mask = mask | bit_for_number(number);
                            next[new_mask as usize * 7 + j as usize] += cur * p;
                        }
                    } else {
                        // Devil die: face 0 is Joker, faces 1..5 -> numbers 2..6
                        let p_joker = die.probs[0];
                        if p_joker != 0.0 && j < 6 {
                            next[mask as usize * 7 + (j + 1) as usize] += cur * p_joker;
                        }

                        for face in 1..6 {
                            let p = die.probs[face];
                            if p == 0.0 {
                                continue;
                            }
                            let number = (face as u8) + 1; // face 1->2, ..., face 5->6
                            let new_mask = mask | bit_for_number(number);
                            next[new_mask as usize * 7 + j as usize] += cur * p;
                        }
                    }
                }
            }

            dp = next;
        }
    }

    // Required masks:
    // 1-5 => {1,2,3,4,5}
    // 2-6 => {2,3,4,5,6}
    // 1-6 => {1,2,3,4,5,6}
    let req_1to5: u8 = bit_for_number(1) | bit_for_number(2) | bit_for_number(3) | bit_for_number(4) | bit_for_number(5);
    let req_2to6: u8 = bit_for_number(2) | bit_for_number(3) | bit_for_number(4) | bit_for_number(5) | bit_for_number(6);
    let req_1to6: u8 = req_1to5 | bit_for_number(6);

    #[inline]
    fn popcount_u8(x: u8) -> u32 {
        x.count_ones()
    }

    let mut p_1to5 = 0.0;
    let mut p_2to6 = 0.0;
    let mut p_1to6 = 0.0;

    for mask in 0..64u8 {
        for j in 0..=6u8 {
            let pr = dp[mask as usize * 7 + j as usize];
            if pr == 0.0 {
                continue;
            }

            let missing_1to5 = popcount_u8(req_1to5 & !mask) as u8;
            let missing_2to6 = popcount_u8(req_2to6 & !mask) as u8;
            let missing_1to6 = popcount_u8(req_1to6 & !mask) as u8;

            if missing_1to5 <= j { p_1to5 += pr; }
            if missing_2to6 <= j { p_2to6 += pr; }
            if missing_1to6 <= j { p_1to6 += pr; }
        }
    }

    (p_1to5, p_2to6, p_1to6)
}

#[derive(Debug, Clone)]
pub struct StraightTerm {
    pub name: &'static str,
    pub p: f64,
    pub score: u32,
    pub ev: f64,
}

pub fn print_straight_breakdown(terms: &[StraightTerm]) {
    for t in terms {
        println!(
            "{}  p={:.2}%  score={}  ev={:.3}",
            t.name,
            t.p * 100.0,
            t.score,
            t.ev
        );
    }
}


pub fn straight_terms_exclusive(dice: &[Die], counts: &[u8]) -> Vec<StraightTerm> {
    let (p15, p26, p16) = probabilities_straights(dice, counts);

    let p26_only = (p26 - p16).max(0.0);
    let p15_only = (p15 - p16).max(0.0);

    vec![
        StraightTerm { name: "Straight 1-6",      p: p16,     score: 1500, ev: p16 * 1500.0 },
        StraightTerm { name: "Straight 2-6 only", p: p26_only,score: 750,  ev: p26_only * 750.0 },
        StraightTerm { name: "Straight 1-5 only", p: p15_only,score: 500,  ev: p15_only * 500.0 },
    ]
}
