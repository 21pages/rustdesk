use super::*;

fn reply(qos: &mut VideoQoS, id: i32, delay: u32) {
    qos.advance_ms(delay.max(1000) as u64);
    let bitrate = (6000.0 * qos.ratio()) as u32;
    qos.store_bitrate(bitrate);
    qos.user_network_delay(id, delay);
    let bitrate = (6000.0 * qos.ratio()) as u32;
    qos.store_bitrate(bitrate);
    qos.update_display_data("threshold", qos.fps() as usize);
}

fn warmed(base: u32, cap: u32, abr: bool) -> VideoQoS {
    let mut qos = super::smoke::session(cap, Quality::Balanced);
    qos.abr_config = abr;
    qos.new_display("threshold".to_owned());
    qos.set_support_changing_quality("threshold", true);
    for _ in 0..40 {
        reply(&mut qos, 1, base);
    }
    assert_eq!(qos.fps(), cap);
    qos
}

#[test]
fn high_baseline_jitter_preserves_fps_and_bitrate() {
    println!("| baseline ms | ABR | cap | min FPS | mean FPS | min ratio / target |");
    println!("|---:|:---:|---:|---:|---:|---:|");
    let mut failures = Vec::new();
    for base in [20, 159, 500, 800, 1000, 1200, 1500] {
        for abr in [false, true] {
            for cap in [30, 60] {
                let mut qos = warmed(base, cap, abr);
                let target_ratio = qos.ratio();
                let mut min_ratio = target_ratio;
                let mut trace = Vec::new();
                // Bursts of six fresh replies, with a low sample between bursts.
                // The normal floor remains visible; baseline relearning cannot hide cuts.
                for i in 0..56 {
                    let extra = if i % 7 == 0 { 0 } else { base.min(1200) / 5 };
                    reply(&mut qos, 1, base + extra);
                    trace.push(qos.fps());
                    min_ratio = min_ratio.min(qos.ratio());
                }
                let min_fps = *trace.iter().min().unwrap();
                let mean = trace.iter().sum::<u32>() as f64 / trace.len() as f64;
                println!(
                    "| {base} | {abr} | {cap} | {min_fps} | {mean:.2} | {:.3} |",
                    min_ratio / target_ratio
                );
                if min_fps < cap || min_ratio < target_ratio * 0.999 {
                    failures.push(format!(
                        "base={base} ABR={abr} cap={cap}: FPS={trace:?}, ratio={min_ratio}"
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "healthy jitter was penalized: {failures:#?}"
    );
}

#[test]
fn recovery_uses_the_same_high_baseline_threshold() {
    let mut qos = warmed(1000, 30, true);
    for _ in 0..8 {
        reply(&mut qos, 1, 1600);
    }
    assert!(qos.fps() < 30);
    let reduced_ratio = qos.ratio();
    reply(&mut qos, 1, 1200);
    assert!(qos.fps() < 30, "one reply must not restore the cap");
    reply(&mut qos, 1, 1200);
    assert_eq!(
        qos.fps(),
        30,
        "two replies below the relative threshold restore FPS"
    );
    for _ in 0..30 {
        reply(&mut qos, 1, 1200);
    }
    assert!(
        qos.ratio() > reduced_ratio,
        "bitrate recovery must accept the same replies"
    );
}

#[test]
fn one_viewers_baseline_does_not_relax_another_viewers_threshold() {
    let mut qos = warmed(1000, 60, true);
    let mut low = warmed(20, 30, true);
    qos.users.insert(1652, low.users.remove(&1).unwrap());
    for _ in 0..4 {
        reply(&mut qos, 1, 1200);
    }
    assert_eq!(qos.users[&1].delay.fps, Some(60));
    let before = qos.ratio();
    // The high-baseline viewer has the larger raw excess (200 vs 170), but only
    // the low-baseline viewer should prevent recovery and then request a cut.
    reply(&mut qos, 1652, 190);
    let after_first = qos.ratio();
    assert!(
        after_first >= before,
        "one fresh bad reply cannot cut bitrate"
    );
    reply(&mut qos, 1652, 190);
    assert!(qos.ratio() < after_first);
    assert_eq!(qos.users[&1].delay.fps, Some(60));
    assert_eq!(qos.users[&1652].delay.consecutive_bad_samples, 2);
}

#[test]
fn relative_threshold_still_reacts_to_sustained_congestion() {
    println!("| baseline ms | ABR | first cut sample | min FPS | recovery replies |");
    println!("|---:|:---:|---:|---:|---:|");
    for base in [20, 500, 800, 1000, 1200, 1500] {
        for abr in [false, true] {
            let mut qos = warmed(base, 30, abr);
            let start_ratio = qos.ratio();
            let mut first_cut = None;
            let mut min_fps = 30;
            for sample in 1..=12 {
                // A growing queue must not become a stable replacement baseline.
                reply(&mut qos, 1, base + 350 + sample * 30);
                if qos.fps() < 30 || qos.ratio() < start_ratio {
                    first_cut.get_or_insert(sample);
                }
                min_fps = min_fps.min(qos.fps());
            }
            assert!(
                first_cut.is_some_and(|sample| sample <= 4),
                "base={base} ABR={abr}"
            );
            assert!(min_fps >= 5);
            assert!(
                min_fps < 30,
                "sustained congestion must eventually reduce FPS"
            );
            reply(&mut qos, 1, base);
            assert!(qos.fps() < 30);
            reply(&mut qos, 1, base);
            assert_eq!(qos.fps(), 30);
            println!(
                "| {base} | {abr} | {} | {min_fps} | 2 |",
                first_cut.unwrap()
            );
        }
    }
}

#[test]
fn threshold_bounds_and_fresh_sample_boundary() {
    for (base, threshold) in [
        (20, 150),
        (600, 150),
        (800, 200),
        (1000, 250),
        (1200, 300),
        (1500, 300),
    ] {
        let mut qos = warmed(base, 30, false);
        assert_eq!(qos.users[&1].delay.delay_threshold(), threshold);
        for _ in 0..6 {
            reply(&mut qos, 1, base + threshold - 1);
            assert_eq!(qos.users[&1].delay.consecutive_bad_samples, 0);
            assert_eq!(qos.fps(), 30);
        }
        for bad in 1..=3 {
            reply(&mut qos, 1, base + threshold);
            assert_eq!(qos.users[&1].delay.consecutive_bad_samples, bad);
        }
    }
    let mut delay = UserDelay::default();
    delay.rtt_calculator.update(u32::MAX);
    assert_eq!(delay.delay_threshold(), 300);
    assert_eq!(
        delay.normalized_avg_delay(),
        150,
        "missing samples must prevent bitrate growth"
    );
}

#[test]
fn bitrate_recovery_compares_each_viewers_relative_delay() {
    let mut qos = warmed(1000, 30, true);
    let mut low = warmed(20, 30, true);
    qos.users.insert(1652, low.users.remove(&1).unwrap());
    reply(&mut qos, 1, 1200);
    reply(&mut qos, 1, 1200);
    reply(&mut qos, 1652, 340);
    reply(&mut qos, 1652, 20);
    assert_eq!(qos.users[&1652].delay.consecutive_bad_samples, 0);
    assert_eq!(qos.users[&1652].delay.avg_delay(), 160);
    assert_eq!(qos.users[&1].delay.avg_delay(), 200);
    qos.ratio = BR_BALANCED / 2.0;
    let reduced = qos.ratio();
    qos.adjust_ratio(true);
    assert_eq!(
        qos.ratio(),
        reduced,
        "the smaller raw excess still exceeds its own threshold"
    );
    reply(&mut qos, 1652, 20);
    qos.advance_ms(4000);
    qos.adjust_ratio(true);
    assert!(
        qos.ratio() > reduced,
        "200 ms excess is healthy for the remaining high-baseline viewer"
    );
}

#[test]
fn relative_threshold_does_not_weaken_emergency_fps_brakes_or_user_caps() {
    for base in [20, 800, 1000, 1500] {
        for cap in [1, 5, 15, 30, 60, 120] {
            let mut reference = warmed(20, cap, true);
            reply(&mut reference, 1, 1020);
            let mut qos = warmed(base, cap, true);
            reply(&mut qos, 1, base + 1000);
            assert_eq!(qos.fps(), reference.fps(), "base={base}, cap={cap}");
            for _ in 0..12 {
                reply(&mut qos, 1, base + 1000);
                assert!((cap.min(5)..=cap).contains(&qos.fps()));
            }
            qos.user_delay_response_elapsed(1, 5000);
            assert_eq!(qos.fps(), cap.min(5));
        }
    }
}

#[test]
fn emergency_fps_response_matches_low_baseline_with_and_without_abr() {
    for base in [800, 1000, 1500] {
        for abr in [false, true] {
            for cap in [5, 12, 15, 30, 60] {
                for restored in [false, true] {
                    let mut reference = warmed(20, cap, abr);
                    let mut qos = warmed(base, cap, abr);
                    if restored {
                        // Put both sessions through an actual cut and two good replies.
                        for _ in 0..3 {
                            reply(&mut reference, 1, 1420);
                            reply(&mut qos, 1, base + 1400);
                        }
                        for _ in 0..2 {
                            reply(&mut reference, 1, 20);
                            reply(&mut qos, 1, base);
                        }
                        assert_eq!(reference.fps(), cap);
                        assert_eq!(qos.fps(), cap);
                    }
                    let excess = if restored { 600 } else { 1000 };
                    reply(&mut reference, 1, 20 + excess);
                    reply(&mut qos, 1, base + excess);
                    assert_eq!(
                        qos.fps(),
                        reference.fps(),
                        "base={base}, cap={cap}, ABR={abr}, restored={restored}"
                    );
                }
            }
        }
    }
}

#[test]
fn high_baseline_closed_loop_smoke() {
    use super::sim::{self, Content, EncoderModel, Link, Scenario, Summary};

    println!("| baseline ms | model | bandwidth | mean target | delivered | queue p95 p90 ms | worst recovery ms |");
    println!("|---:|---|---|---:|---:|---:|---:|");
    let mut failures = Vec::new();
    for base in [500, 800, 1000, 1500] {
        for (model, encoder) in [
            ("CBR", EncoderModel::Cbr),
            ("fixed", EncoderModel::FixedRate),
        ] {
            for drop in [false, true] {
                let sc = Scenario {
                    name: "relative_threshold",
                    seconds: 180,
                    limit: 30,
                    quality: Quality::Balanced,
                    abr: true,
                    content: Content::Video,
                    encoder,
                    link: Link {
                        capacity_kbps: if drop {
                            vec![(0, 8000.0), (60_000, 2500.0), (120_000, 8000.0)]
                        } else {
                            vec![(0, 8000.0)]
                        },
                        wobble: 0.05,
                        base_rtt_ms: base as f64,
                        jitter_median_ms: 5.0,
                        jitter_sigma: 0.5,
                        loss_per_s: 0.0,
                        stall_mean_interval_s: 0.0,
                        stall_ms: (0.0, 0.0),
                    },
                    seed: 1,
                };
                let reports: Vec<_> = (1..=20)
                    .map(|seed| sim::run(&Scenario { seed, ..sc.clone() }))
                    .collect();
                let summary = Summary::of(&reports);
                println!(
                    "| {base} | {model} | {} | {:.2} | {:.2} | {} | {:?} |",
                    if drop { "8 -> 2.5 -> 8 Mbps" } else { "8 Mbps" },
                    summary.mean_target_median,
                    summary.delivered_median,
                    summary.queue_p95_p90,
                    summary.recovery_worst_ms
                );
                assert!(reports
                    .iter()
                    .all(|r| r.min_target_fps >= 5 && r.final_fps == 30));
                if drop {
                    // CBR needs bitrate feedback to drain. At 1-1.5 s RTT the old
                    // controller already exceeds 4 s of estimated queue in this model.
                    // Guard that regression surface separately from delivered frame age.
                    let queue_budget = if encoder == EncoderModel::Cbr {
                        8000
                    } else {
                        3000
                    };
                    if summary.queue_p95_p90 >= queue_budget {
                        failures.push(format!("base={base} model={model}: {summary:?}"));
                    }
                    assert!(summary.frame_age_p95_p90 < 3000, "{summary:?}");
                    assert!(summary.delivered_median >= 22.5, "{summary:?}");
                    assert!(
                        summary.recovery_worst_ms.is_some_and(|ms| ms <= 20_000),
                        "{summary:?}"
                    );
                } else {
                    assert_eq!(summary.p10_target_worst, 30, "{summary:?}");
                    assert!(summary.delivered_median >= 29.0, "{summary:?}");
                    assert!(summary.queue_p95_p90 < 100, "{summary:?}");
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
