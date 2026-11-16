use std::process::{Command, Output};

fn rsea(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rsea"))
        .args(args)
        .env("NO_COLOR", "1")
        .env("TERM", "dumb")
        .output()
        .expect("run rsea")
}

#[test]
fn help_is_available_without_a_terminal() {
    let output = rsea(&["--help"]);
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for option in [
        "--theme",
        "--sea",
        "--speed",
        "--ascii",
        "--seed",
        "--viewpoint",
        "--detail",
    ] {
        assert!(help.contains(option), "missing {option}");
    }
}

#[test]
fn invalid_animation_values_are_rejected_before_entering_the_terminal() {
    for args in [
        ["--speed", "0"],
        ["--speed", "NaN"],
        ["--speed", "inf"],
        ["--intensity", "-1"],
        ["--intensity", "NaN"],
        ["--theme", "rainbow"],
    ] {
        let output = rsea(&args);
        assert!(!output.status.success(), "accepted {args:?}");
        assert!(
            output.stdout.is_empty(),
            "wrote terminal controls on failure"
        );
        assert!(!output.stderr.contains(&0x1b));
    }
}

#[test]
fn animation_requires_a_terminal_and_offers_a_snapshot() {
    let output = rsea(&[]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("--snapshot")
    );
}

#[test]
fn ascii_snapshots_are_reproducible_and_fit_the_requested_size() {
    let args = [
        "--snapshot",
        "--seed",
        "42",
        "--width",
        "80",
        "--height",
        "24",
    ];
    let first = rsea(&args);
    let second = rsea(&args);
    assert!(first.status.success());
    assert_eq!(first.stdout, second.stdout);
    assert!(first.stdout.is_ascii());
    assert!(!first.stdout.contains(&0x1b));
    let picture = String::from_utf8(first.stdout).unwrap();
    let rows: Vec<_> = picture.lines().collect();
    assert_eq!(rows.len(), 24);
    assert!(rows.iter().all(|row| row.len() == 80));
    assert!(picture.contains('~'), "missing waves");
}

#[test]
fn snapshots_respond_to_time_seed_sea_and_camera_settings() {
    let base = ["--snapshot", "--seed", "42"];
    let initial = rsea(&base).stdout;
    assert!(!initial.is_empty());
    for extra in [
        ["--time", "10"],
        ["--sea", "storm"],
        ["--viewpoint", "raised"],
        ["--detail", "balanced"],
    ] {
        let args: Vec<_> = base.into_iter().chain(extra).collect();
        let output = rsea(&args);
        assert!(output.status.success());
        assert_ne!(initial, output.stdout, "ignored {extra:?}");
    }
    assert_ne!(
        initial,
        rsea(&["--snapshot", "--ascii", "--seed", "43"]).stdout
    );
}

#[test]
fn every_theme_supports_ascii_at_small_and_normal_sizes() {
    for theme in ["terminal", "ocean", "day", "sunset", "night"] {
        for (width, height) in [("1", "1"), ("2", "3"), ("80", "24")] {
            let output = rsea(&[
                "--snapshot",
                "--theme",
                theme,
                "--width",
                width,
                "--height",
                height,
                "--time",
                "86400",
            ]);
            assert!(output.status.success());
            assert!(output.stdout.is_ascii());
            let text = String::from_utf8(output.stdout).unwrap();
            assert_eq!(text.lines().count(), height.parse::<usize>().unwrap());
            assert!(
                text.lines()
                    .all(|line| line.len() == width.parse::<usize>().unwrap())
            );
        }
    }
}

#[test]
fn defaults_select_low_rich_water_with_terminal_colors() {
    let implicit = rsea(&["--snapshot", "--seed", "42", "--color", "truecolor"]);
    let explicit = rsea(&[
        "--snapshot",
        "--seed",
        "42",
        "--theme",
        "terminal",
        "--viewpoint",
        "low",
        "--detail",
        "rich",
        "--color",
        "truecolor",
    ]);
    assert!(implicit.status.success());
    assert!(explicit.status.success());
    assert_eq!(implicit.stdout, explicit.stdout);
    assert!(implicit.stdout.is_ascii());
    assert!(
        !implicit.stdout.contains(&0x1b),
        "default theme changed terminal colors"
    );
}

#[test]
fn renderer_matches_the_approved_browser_scene() {
    let output = rsea(&[
        "--snapshot",
        "--seed",
        "42",
        "--width",
        "91",
        "--height",
        "32",
        "--time",
        "5",
    ]);
    assert!(output.status.success());
    let picture = String::from_utf8(output.stdout).unwrap();
    let actual: Vec<_> = picture.lines().map(str::trim_end).collect();
    let expected: Vec<_> = include_str!("fixtures/open-water-low-rich.txt")
        .lines()
        .collect();
    assert_eq!(actual.len(), expected.len());
    for (row, (actual, expected)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(*actual, expected, "approved preview differs at row {row}");
    }
}

#[test]
fn themes_change_colors_without_adding_objects_or_changing_the_water() {
    let base = rsea(&["--snapshot", "--seed", "42", "--time", "5"]);
    let colored = rsea(&[
        "--snapshot",
        "--seed",
        "42",
        "--time",
        "5",
        "--color",
        "truecolor",
    ]);
    for theme in ["ocean", "day", "sunset", "night"] {
        let picture = rsea(&[
            "--snapshot",
            "--seed",
            "42",
            "--time",
            "5",
            "--theme",
            theme,
        ]);
        assert!(picture.status.success());
        assert!(
            picture.stdout == base.stdout,
            "{theme} changed scene geometry"
        );
        let themed = rsea(&[
            "--snapshot",
            "--seed",
            "42",
            "--time",
            "5",
            "--theme",
            theme,
            "--color",
            "truecolor",
        ]);
        assert!(themed.status.success());
        assert_ne!(themed.stdout, colored.stdout);
        assert!(
            !String::from_utf8(themed.stdout)
                .unwrap()
                .contains("\x1b[48;"),
            "{theme} painted a background"
        );
    }
}

#[test]
fn zero_intensity_produces_still_flat_water() {
    let first = rsea(&["--snapshot", "--seed", "42", "--intensity", "0"]);
    let later = rsea(&[
        "--snapshot",
        "--seed",
        "42",
        "--intensity",
        "0",
        "--time",
        "100",
    ]);
    assert!(first.status.success());
    assert!(later.status.success());
    assert!(first.stdout == later.stdout, "flat water changed with time");
}

#[test]
fn snapshot_dimensions_and_time_are_bounded() {
    for args in [
        ["--width", "0"],
        ["--height", "0"],
        ["--width", "501"],
        ["--height", "201"],
        ["--time", "NaN"],
        ["--time", "86401"],
    ] {
        let output = rsea(&["--snapshot", args[0], args[1]]);
        assert!(!output.status.success(), "accepted {args:?}");
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn explicit_color_overrides_no_color_without_adding_cursor_controls_to_snapshots() {
    for (mode, marker) in [("truecolor", "38;2;"), ("ansi", "38;5;")] {
        let output = rsea(&["--snapshot", "--theme", "ocean", "--color", mode]);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains(marker));
        assert!(!text.contains("\x1b[48;"), "palette painted a background");
        assert!(!text.contains("?1049"), "snapshot entered alternate screen");
        assert!(!text.contains("?25"), "snapshot changed cursor visibility");
    }
}
