#[derive(Clone)]
pub struct Die {
    //weights: [u32; 6],
    probs: [f64; 6],
    name: String,
}

impl Die {
    pub fn new(weights: [u32; 6], name: String) -> Self {
        let sum: u32 = weights.iter().sum();

        assert!(sum > 0, "Die weights must not sum to zero");

        let sum_f = sum as f64;

        let probs = [
            weights[0] as f64 / sum_f,
            weights[1] as f64 / sum_f,
            weights[2] as f64 / sum_f,
            weights[3] as f64 / sum_f,
            weights[4] as f64 / sum_f,
            weights[5] as f64 / sum_f,
        ];

        Self { probs, name }
    }
}


fn probability_exact(
    dice: &[Die],
    counts: &[u8],
    face: usize,
    target: usize,
) -> f64 {
    let mut dp = [0.0f64; 7];
    dp[0] = 1.0;

    for (die_index, &count) in counts.iter().enumerate() {
        let p = dice[die_index].probs[face];
        let q = 1.0 - p;

        for _ in 0..count {
            let mut next = [0.0f64; 7];

            for r in 0..=6 {
                if dp[r] == 0.0 {
                    continue;
                }

                // nepadne
                next[r] += dp[r] * q;

                // padne
                if r + 1 <= 6 {
                    next[r + 1] += dp[r] * p;
                }
            }

            dp = next;
        }
    }

    dp[target]
}

#[derive(Debug, Clone)]
pub struct EvTerm {
    pub face: usize,
    // 0..5 (1..6)
    pub count: usize,
    // 0..6
    pub p: f64,
    pub score: u32,
    pub ev: f64,
}

/// Expected score of the *first roll* (one throw of 6 dice),
/// ignoring straights and Devil's Head, using KCD2 scoring:
/// - single 1 = 100
/// - single 5 = 50
/// - three-of-a-kind: 1..6 => {1000,200,300,400,500,600}
/// - each additional die after 3 doubles the value (4x = 2×, 5x = 4×, 6x = 8×)
///
/// This assumes scoring is additive across faces (true here since we ignore straights/jokers).
pub fn expected_first_roll_score(
    dice: &[Die],
    counts: &[u8],
) -> f64 {
    debug_assert_eq!(
        counts.iter().map(|&c| c as usize).sum::<usize>(),
        6,
        "counts must sum to 6 dice"
    );

    let mut total_ev = 0.0;

    for face in 0..6 {
        for target in 0..=6 {
            let s = score_for_face_count(face, target);
            if s == 0 {
                continue;
            }

            let p = probability_exact(dice, counts, face, target);
            let ev = p * s as f64;
            total_ev += ev;
        }
    }

    total_ev
}

/// Score contributed by a single face (0..5 == 1..6) given that it appears `count` times in the roll.
/// Returns an integer score (points).
fn score_for_face_count(face: usize, count: usize) -> u32 {
    // face: 0..5 represents value 1..6
    let value = face + 1;

    // Singles
    if value == 1 && count < 3 {
        return (count as u32) * 100;
    }
    if value == 5 && count < 3 {
        return (count as u32) * 50;
    }

    // Three-of-a-kind base values
    let base3: u32 = match value {
        1 => 1000,
        2 => 200,
        3 => 300,
        4 => 400,
        5 => 500,
        6 => 600,
        _ => unreachable!(),
    };

    if count < 3 {
        return 0;
    }

    // After 3, each additional die doubles the value
    let multiplier: u32 = 1u32 << ((count - 3) as u32); // 3->1, 4->2, 5->4, 6->8
    base3 * multiplier
}

pub fn expected_first_roll_score_with_breakdown(
    dice: &[Die],
    counts: &[u8],
) -> (f64, Vec<EvTerm>) {
    debug_assert_eq!(
        counts.iter().map(|&c| c as usize).sum::<usize>(),
        6,
        "counts must sum to 6 dice"
    );

    let mut total_ev = 0.0;
    let mut terms: Vec<EvTerm> = Vec::new();

    for face in 0..6 {
        for count in 0..=6 {
            let score = score_for_face_count(face, count);
            if score == 0 {
                continue;
            }

            let p = probability_exact(dice, counts, face, count);
            if p == 0.0 {
                continue;
            }

            let ev = p * score as f64;
            total_ev += ev;

            terms.push(EvTerm {
                face,
                count,
                p,
                score,
                ev,
            });
        }
    }

    // Sort descending by EV contribution
    terms.sort_by(|a, b| b.ev.partial_cmp(&a.ev).unwrap_or(std::cmp::Ordering::Equal));

    (total_ev, terms)
}

/// Convenience printer (top N)
pub fn print_ev_breakdown(terms: &[EvTerm], top_n: usize) {
    for (_i, t) in terms.iter().take(top_n).enumerate() {
        println!(
            "{}x{}  p={:.2}%  score={}  ev={:.3}",
            t.count,
            t.face + 1,
            t.p * 100.0,
            t.score,
            t.ev
        );
    }
}

pub fn format_dice_set(dice_types: &[Die], counts: &[u8]) -> String {
    assert_eq!(dice_types.len(), counts.len());

    let mut parts: Vec<String> = Vec::new();

    for (die, &count) in dice_types.iter().zip(counts.iter()) {
        if count > 0 {
            parts.push(format!("{}x {}", count, die.name));
        }
    }

    parts.join(", ")
}

pub fn evaluate_and_print(dice_types: &[Die], counts: &[u8]) {
    assert_eq!(
        dice_types.len(),
        counts.len(),
        "dice_types and counts must have same length"
    );

    let (ev, breakdown) =
        expected_first_roll_score_with_breakdown(dice_types, counts);

    let description = format_dice_set(dice_types, counts);

    println!("{} = {:.2}", description, ev);

    print_ev_breakdown(&breakdown, usize::MAX);

    println!("---------------------------------------------");
}

fn main() {
    let default_die = Die::new([1, 1, 1, 1, 1, 1], "Default".to_string());
    let arranka_die = Die::new([6, 1, 6, 1, 6, 1], "Arranka".to_string());
    let unbalanced_die = Die::new([3, 4, 1, 1, 2, 1], "Unbalanced".to_string());
    let trinity_die = Die::new([4, 5, 10, 1, 1, 1], "Trinity".to_string());
    let misforutne_die = Die::new([1, 5, 5, 5, 5, 1], "Misfortune".to_string());

    let dice_types = [default_die.clone()];
    let counts = [6u8];
    evaluate_and_print(&dice_types, &counts);

    let dice_types = [default_die.clone(), arranka_die.clone()];
    let counts = [5, 1];
    evaluate_and_print(&dice_types, &counts);

    let dice_types = [default_die.clone(), arranka_die.clone(), unbalanced_die.clone()];
    let counts = [4, 1, 1];
    evaluate_and_print(&dice_types, &counts);

    let dice_types = [default_die.clone(), arranka_die.clone(), unbalanced_die.clone(), trinity_die.clone()];
    let counts = [3, 1, 1, 1];
    evaluate_and_print(&dice_types, &counts);

    let dice_types = [default_die.clone(), arranka_die.clone(), unbalanced_die.clone(), trinity_die.clone()];
    let counts = [2, 1, 1, 2];
    evaluate_and_print(&dice_types, &counts);

    let dice_types = [default_die.clone(), arranka_die.clone(), unbalanced_die.clone(), trinity_die.clone(), misforutne_die.clone()];
    let counts = [1, 1, 1, 2, 1];
    evaluate_and_print(&dice_types, &counts);
}
