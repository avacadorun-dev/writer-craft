//! Compiled-widget probes; exact native appearance and OS input need separate review.
use super::*;
use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
use std::collections::BTreeMap;

fn projects(count: usize) -> Vec<Project> {
    (0..count)
        .map(|index| Project {
            id: format!("project-{index}"),
            title: "A duplicate title — Длинное название 日本語".repeat(5),
        })
        .collect()
}

fn context() -> egui::Context {
    let ctx = egui::Context::default();
    super::super::install_fonts(&ctx);
    ctx
}

fn frame(
    ctx: &egui::Context,
    size: Vec2,
    projects: &[Project],
    resume: Option<&Project>,
    language: Language,
    settings: ThemeSettings,
    events: Vec<Event>,
) -> Option<view::Action> {
    let ticks = ctx.data_mut(|data| {
        let ticks = data.get_temp_mut_or_default::<u32>(egui::Id::new("home-test-ticks"));
        *ticks += 1;
        *ticks
    });
    ctx.data_mut(|data| {
        data.insert_temp(
            egui::Id::new("home-test-geometry"),
            BTreeMap::<String, Rect>::new(),
        )
    });
    let tokens = theme::apply(ctx, &settings);
    let mut action = None;
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            time: Some(f64::from(ticks) * 0.1),
            events,
            ..Default::default()
        },
        |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                action = view::show(ui, projects, resume, language, tokens);
            });
        },
    );
    output.textures_delta.clear();
    action
}

#[test]
fn tab_through_long_library_reveals_rows_and_shift_tab_returns_to_banner() {
    let ctx = context();
    let items = projects(30);
    let size = Vec2::new(1000.0, 440.0);
    let idle = |events| {
        let _ = frame(
            &ctx,
            size,
            &items,
            None,
            Language::English,
            ThemeSettings::default(),
            events,
        );
    };
    for _ in 0..3 {
        idle(vec![]);
    }
    let Some(row_id) =
        ctx.data(|data| data.get_temp::<egui::Id>(egui::Id::new(("home-test-row", "project-29"))))
    else {
        panic!("last row emitted");
    };
    let Some(primary_id) = ctx.data(|data| {
        data.get_temp::<egui::Id>(egui::Id::new(("home-test-control", "home.primary")))
    }) else {
        panic!("primary emitted");
    };
    let tab = |shift| {
        let modifiers = if shift {
            Modifiers::SHIFT
        } else {
            Modifiers::NONE
        };
        idle(vec![Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }]);
        idle(vec![Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers,
        }]);
        for _ in 0..3 {
            idle(vec![]);
        }
    };
    for _ in 0..40 {
        tab(false);
        if ctx.memory(|memory| memory.has_focus(row_id)) {
            break;
        }
    }
    assert!(
        ctx.memory(|memory| memory.has_focus(row_id)),
        "Tab never reached the last row"
    );
    assert!(
        Rect::from_min_size(Pos2::ZERO, size)
            .contains_rect(geometry(&ctx)["home.project.project-29"]),
        "focused last row stayed clipped"
    );
    for _ in 0..40 {
        tab(true);
        if ctx.memory(|memory| memory.has_focus(primary_id)) {
            break;
        }
    }
    assert!(
        ctx.memory(|memory| memory.has_focus(primary_id)),
        "Shift+Tab never returned to the banner action"
    );
    assert!(
        Rect::from_min_size(Pos2::ZERO, size).contains_rect(geometry(&ctx)["home.primary"]),
        "focused banner action stayed clipped"
    );
}

fn geometry(ctx: &egui::Context) -> BTreeMap<String, Rect> {
    ctx.data(|data| data.get_temp::<BTreeMap<String, Rect>>(egui::Id::new("home-test-geometry")))
        .unwrap_or_default()
}

fn click(ctx: &egui::Context, items: &[Project], target: Pos2) -> Option<view::Action> {
    let size = Vec2::new(1000.0, 800.0);
    let _ = frame(
        ctx,
        size,
        items,
        None,
        Language::English,
        ThemeSettings::default(),
        vec![
            Event::PointerMoved(target),
            Event::PointerButton {
                pos: target,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    frame(
        ctx,
        size,
        items,
        None,
        Language::English,
        ThemeSettings::default(),
        vec![Event::PointerButton {
            pos: target,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }],
    )
}

#[test]
fn continuation_requires_the_actual_current_project_id_not_a_title_match() {
    let items = projects(2);
    let stale = Project {
        id: "missing-project".into(),
        title: items[0].title.clone(),
    };
    assert!(continuation(&items, None).is_none());
    assert!(continuation(&items, Some(&stale)).is_none());
    assert_eq!(
        continuation(&items, Some(&items[1])).map(|p| p.id.as_str()),
        Some("project-1")
    );
}

#[test]
fn duplicate_title_rows_open_their_exact_project_ids() {
    let ctx = context();
    let items = projects(2);
    for _ in 0..3 {
        let _ = frame(
            &ctx,
            Vec2::new(1000.0, 800.0),
            &items,
            None,
            Language::English,
            ThemeSettings::default(),
            vec![],
        );
    }
    let second = geometry(&ctx)["home.project.project-1"].center();
    assert_eq!(
        click(&ctx, &items, second),
        Some(view::Action::Open("project-1".into()))
    );
}

#[test]
fn focused_project_row_opens_by_keyboard() {
    let ctx = context();
    let items = projects(1);
    for _ in 0..3 {
        let _ = frame(
            &ctx,
            Vec2::new(1000.0, 800.0),
            &items,
            None,
            Language::English,
            ThemeSettings::default(),
            vec![],
        );
    }
    let Some(id) =
        ctx.data(|data| data.get_temp::<egui::Id>(egui::Id::new(("home-test-row", "project-0"))))
    else {
        panic!("row emitted");
    };
    ctx.memory_mut(|memory| memory.request_focus(id));
    let action = frame(
        &ctx,
        Vec2::new(1000.0, 800.0),
        &items,
        None,
        Language::English,
        ThemeSettings::default(),
        vec![Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert_eq!(action, Some(view::Action::Open("project-0".into())));
}

#[test]
fn empty_library_keeps_creation_import_and_sample_controls_reachable() {
    let ctx = context();
    for _ in 0..3 {
        let _ = frame(
            &ctx,
            Vec2::new(1000.0, 800.0),
            &[],
            None,
            Language::English,
            ThemeSettings::default(),
            vec![],
        );
    }
    let observed = geometry(&ctx);
    assert_eq!(
        click(&ctx, &[], observed["home.primary"].center()),
        Some(view::Action::New)
    );
    assert_eq!(
        click(&ctx, &[], observed["home.import"].center()),
        Some(view::Action::Import)
    );
    assert_eq!(
        click(&ctx, &[], observed["home.sample"].center()),
        Some(view::Action::Sample)
    );
}

#[test]
fn localized_resizing_contains_artwork_controls_and_project_rows() {
    let ctx = context();
    let items = projects(3);
    for language in Language::ALL {
        for size in [
            Vec2::new(360.0, 480.0),
            Vec2::new(620.0, 720.0),
            Vec2::new(760.0, 420.0),
            Vec2::new(1200.0, 900.0),
        ] {
            for _ in 0..3 {
                let _ = frame(
                    &ctx,
                    size,
                    &items,
                    Some(&items[0]),
                    language,
                    ThemeSettings::default(),
                    vec![],
                );
            }
            let observed = geometry(&ctx);
            let banner = observed["home.banner"];
            assert!(
                banner.contains_rect(observed["home.artwork"]),
                "art escaped {language:?} {size:?}: {observed:?}"
            );
            for (key, rect) in observed {
                assert!(
                    rect.left() >= 0.0 && rect.right() <= size.x + 1.0,
                    "horizontal overflow {language:?} {size:?} {key}: {rect:?}"
                );
            }
        }
    }
}

#[test]
fn home_stays_inside_the_real_central_body_after_shared_panel_resizing() {
    let ctx = context();
    let items = projects(3);
    let tokens = theme::apply(&ctx, &ThemeSettings::default());
    let mut tick = 0;
    for language in [Language::Russian, Language::German, Language::Japanese] {
        // Reuse one Context: panel placement and wrapping must survive live
        // resizing, not merely a zero-origin standalone render.
        for (size, navigator) in [
            (Vec2::new(1000.0, 680.0), 264.0),
            (Vec2::new(1440.0, 940.0), 210.0),
            (Vec2::new(1280.0, 800.0), 340.0),
            (Vec2::new(1000.0, 680.0), 340.0),
            (Vec2::new(1000.0, 520.0), 264.0),
        ] {
            let mut body = Rect::NOTHING;
            for _ in 0..3 {
                tick += 1;
                ctx.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new("home-test-geometry"),
                        BTreeMap::<String, Rect>::new(),
                    )
                });
                let mut output = ctx.run_ui(
                    RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                        time: Some(f64::from(tick) * 0.1),
                        ..Default::default()
                    },
                    |ui| {
                        egui::Panel::top("home-test-header")
                            .exact_size(48.0)
                            .show(ui, |_| {});
                        egui::Panel::bottom("home-test-footer")
                            .exact_size(28.0)
                            .show(ui, |_| {});
                        egui::Panel::left("home-test-navigator")
                            .exact_size(navigator)
                            .show(ui, |_| {});
                        egui::CentralPanel::default()
                            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(20, 20)))
                            .show(ui, |ui| {
                                body = ui.available_rect_before_wrap();
                                let _ = view::show(ui, &items, Some(&items[0]), language, tokens);
                            });
                    },
                );
                output.textures_delta.clear();
            }
            let observed = geometry(&ctx);
            assert!(
                body.left() >= navigator + 20.0,
                "fixture did not reserve shared shell"
            );
            for (key, rect) in &observed {
                assert!(
                    rect.left() >= body.left() - 1.0 && rect.right() <= body.right() + 1.0,
                    "body intrusion {language:?} {size:?} nav={navigator}: {key} {rect:?}, body={body:?}"
                );
            }
            let banner = observed["home.banner"];
            let artwork = observed["home.artwork"];
            assert!(banner.contains_rect(artwork));
            if banner.width() >= 660.0 {
                assert!(
                    (banner.right() - artwork.right()).abs() <= 1.0,
                    "artwork did not reach right column: {banner:?} {artwork:?}"
                );
                assert!(
                    (banner.bottom() - artwork.bottom()).abs() <= 1.0,
                    "artwork was not bottom anchored: {banner:?} {artwork:?}"
                );
            }
            if size.y >= 680.0 {
                assert!(
                    observed["home.project.project-0"].top() < body.bottom(),
                    "banner displaced the first project off screen"
                );
            }
        }
    }
}

#[test]
fn long_library_scroll_moves_the_banner_and_reaches_the_last_project() {
    let ctx = context();
    let items = projects(30);
    let size = Vec2::new(1000.0, 440.0);
    for _ in 0..3 {
        let _ = frame(
            &ctx,
            size,
            &items,
            None,
            Language::English,
            ThemeSettings::default(),
            vec![],
        );
    }
    let initial = geometry(&ctx)["home.banner"].top();
    let _ = frame(
        &ctx,
        size,
        &items,
        None,
        Language::English,
        ThemeSettings::default(),
        vec![
            Event::PointerMoved(egui::pos2(600.0, 300.0)),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(0.0, -10000.0),
                phase: egui::TouchPhase::Move,
                modifiers: Modifiers::NONE,
            },
        ],
    );
    for _ in 0..12 {
        let _ = frame(
            &ctx,
            size,
            &items,
            None,
            Language::English,
            ThemeSettings::default(),
            vec![],
        );
    }
    let observed = geometry(&ctx);
    assert!(observed["home.banner"].top() < initial);
    assert!(Rect::from_min_size(Pos2::ZERO, size).intersects(observed["home.project.project-29"]));
}
