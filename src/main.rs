mod game_logic;

use dioxus::prelude::*;
use game_logic::*;
use rand::Rng;

// ========== Data Types ==========

#[derive(Clone, Copy, Debug, PartialEq)]
enum GameState {
    Playing,
    Revealed,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ActiveMode {
    Interactive,
    Demo,
    Simulation,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Stats {
    total: u64,
    switch_wins: u64,
    stick_wins: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct DataPoint {
    total: u64,
    switch_pct: f64,
    stick_pct: f64,
}

// ========== Main ==========

fn main() {
    dioxus::launch(App);
}

// ========== App Component ==========

#[component]
fn App() -> Element {
    // Game engine state
    let mut doors = use_signal(|| setup_doors());
    let mut game_state = use_signal(|| GameState::Playing);
    let mut player_choice = use_signal(|| None::<usize>);
    let mut initial_choice = use_signal(|| None::<usize>);
    let mut host_choice = use_signal(|| None::<usize>);
    let mut did_win = use_signal(|| None::<bool>);

    // Simulation state
    let mut active_mode = use_signal(|| ActiveMode::Interactive);
    let mut is_running = use_signal(|| false);
    let mut is_visual_demo = use_signal(|| false);

    // Stats per mode
    let mut sim_stats = use_signal(Stats::default);
    let mut sim_history = use_signal(Vec::<DataPoint>::new);
    let mut demo_stats = use_signal(Stats::default);
    let mut demo_history = use_signal(Vec::<DataPoint>::new);
    let mut interactive_stats = use_signal(Stats::default);
    let mut interactive_history = use_signal(Vec::<DataPoint>::new);

    // Active stats/history based on mode
    let active_stats = match *active_mode.read() {
        ActiveMode::Simulation => sim_stats.read().clone(),
        ActiveMode::Demo => demo_stats.read().clone(),
        ActiveMode::Interactive => interactive_stats.read().clone(),
    };

    let active_history = match *active_mode.read() {
        ActiveMode::Simulation => sim_history.read().clone(),
        ActiveMode::Demo => demo_history.read().clone(),
        ActiveMode::Interactive => interactive_history.read().clone(),
    };

    let running = *is_running.read();
    let visual_demo = *is_visual_demo.read();
    let current_game_state = *game_state.read();
    let current_doors = *doors.read();
    let current_player_choice = *player_choice.read();
    let current_host_choice = *host_choice.read();
    let current_did_win = *did_win.read();

    // ---- Callbacks ----

    let mut reset_game = move |_: ()| {
        doors.set(setup_doors());
        game_state.set(GameState::Playing);
        player_choice.set(None);
        initial_choice.set(None);
        host_choice.set(None);
        did_win.set(None);
    };

    let mut handle_door_click = move |door_id: usize| {
        // Read and drop immediately to avoid borrow conflicts
        let gs = current_game_state;
        match gs {
            GameState::Playing => {
                active_mode.set(ActiveMode::Interactive);
                player_choice.set(Some(door_id));
                initial_choice.set(Some(door_id));
                let current = *doors.read();
                let revealed_id = host_reveal(&current, door_id);
                host_choice.set(Some(revealed_id));
                let mut new_doors = current;
                for d in new_doors.iter_mut() {
                    if d.id == revealed_id {
                        d.status = DoorStatus::Open;
                    }
                }
                doors.set(new_doors);
                game_state.set(GameState::Revealed);
            }
            GameState::Revealed => {
                let current = *doors.read();
                let (won, new_doors) = resolve_game(&current, door_id);
                did_win.set(Some(won));
                doors.set(new_doors);
                player_choice.set(Some(door_id));
                game_state.set(GameState::Finished);

                let init = initial_choice.read().unwrap_or(0);
                let did_switch = door_id != init;
                let mut s = interactive_stats.read().clone();
                s.total += 1;
                if did_switch && won {
                    s.switch_wins += 1;
                }
                if !did_switch && won {
                    s.stick_wins += 1;
                }

                interactive_history.write().push(DataPoint {
                    total: s.total,
                    switch_pct: if s.total > 0 { s.switch_wins as f64 / s.total as f64 } else { 0.0 },
                    stick_pct: if s.total > 0 { s.stick_wins as f64 / s.total as f64 } else { 0.0 },
                });
                {
                    let mut h = interactive_history.write();
                    if h.len() > 50 {
                        let start = h.len() - 50;
                        *h = h[start..].to_vec();
                    }
                }
                interactive_stats.set(s);
            }
            GameState::Finished => {}
        }
    };

    // Run simulation
    let mut handle_run_simulation = move |runs: u64| {
        if runs > 1000 {
            // Turbo mode: chunked async batch
            active_mode.set(ActiveMode::Simulation);
            is_running.set(true);
            sim_stats.set(Stats::default());
            sim_history.set(vec![]);

            spawn(async move {
                let chunk_size: u64 = 1_000_000;
                let mut total_done: u64 = 0;
                let mut total_sw: u64 = 0;
                let mut total_stw: u64 = 0;
                
                let mut last_ui_update = web_time::Instant::now();

                while total_done < runs {
                    if !*is_running.read() {
                        break;
                    }

                    let batch = std::cmp::min(chunk_size, runs - total_done);
                    let (sw, stw) = run_batch(batch);
                    total_done += batch;
                    total_sw += sw;
                    total_stw += stw;

                    // Update UI smoothly every ~50ms or when finished
                    if last_ui_update.elapsed().as_millis() > 50 || total_done >= runs {
                        last_ui_update = web_time::Instant::now();
                        
                        sim_stats.set(Stats {
                            total: total_done,
                            switch_wins: total_sw,
                            stick_wins: total_stw,
                        });

                        sim_history.write().push(DataPoint {
                            total: total_done,
                            switch_pct: total_sw as f64 / total_done as f64,
                            stick_pct: total_stw as f64 / total_done as f64,
                        });
                        
                        {
                            let mut h = sim_history.write();
                            if h.len() > 100 {
                                let start = h.len() - 100;
                                *h = h[start..].to_vec();
                            }
                        }
                    }

                    // Always yield to let the browser process the UI updates
                    gloo_timers::future::TimeoutFuture::new(0).await;
                }

                is_running.set(false);
            });
        } else {
            // Visual demo mode
            active_mode.set(ActiveMode::Demo);
            is_visual_demo.set(true);
            demo_stats.set(Stats::default());
            demo_history.set(vec![]);

            spawn(async move {
                let mut current_sw: u64 = 0;
                let mut current_stw: u64 = 0;

                for i in 0..runs {
                    if !*is_visual_demo.read() {
                        break;
                    }

                    // Reset game for visual
                    let new_doors = setup_doors();
                    doors.set(new_doors);
                    game_state.set(GameState::Playing);
                    player_choice.set(None);
                    initial_choice.set(None);
                    host_choice.set(None);
                    did_win.set(None);

                    // Exponential decay delay
                    let min_delay = 10u32;
                    let max_delay = 900u32;
                    let decay: f64 = 0.95;
                    let delay = std::cmp::max(
                        min_delay,
                        (min_delay as f64 + (max_delay as f64 - min_delay as f64) * decay.powi(i as i32)) as u32,
                    );

                    gloo_timers::future::TimeoutFuture::new(delay).await;

                    // 1. Pick a random door
                    let pick = rand::thread_rng().gen_range(0..3usize) + 1;
                    player_choice.set(Some(pick));
                    initial_choice.set(Some(pick));

                    let current = *doors.read();
                    let revealed_id = host_reveal(&current, pick);
                    host_choice.set(Some(revealed_id));
                    let mut d2 = current;
                    for d in d2.iter_mut() {
                        if d.id == revealed_id {
                            d.status = DoorStatus::Open;
                        }
                    }
                    doors.set(d2);
                    game_state.set(GameState::Revealed);

                    gloo_timers::future::TimeoutFuture::new(delay).await;

                    // 2. Randomly switch or stick
                    let will_switch = rand::thread_rng().gen_bool(0.5);
                    let final_pick = if will_switch {
                        (1..=3usize).find(|&d| d != pick && d != revealed_id).unwrap()
                    } else {
                        pick
                    };

                    let current3 = *doors.read();
                    let (won, new_doors3) = resolve_game(&current3, final_pick);
                    did_win.set(Some(won));
                    doors.set(new_doors3);
                    player_choice.set(Some(final_pick));
                    game_state.set(GameState::Finished);

                    // Tally for our local dashboard
                    let car_id = new_doors.iter().find(|d| d.content == DoorContent::Car).unwrap().id;
                    let switch_door = (1..=3usize).find(|&d| d != pick && d != revealed_id).unwrap();
                    if switch_door == car_id {
                        current_sw += 1;
                    }
                    if pick == car_id {
                        current_stw += 1;
                    }

                    let total = i + 1;
                    demo_stats.set(Stats {
                        total,
                        switch_wins: current_sw,
                        stick_wins: current_stw,
                    });

                    demo_history.write().push(DataPoint {
                        total,
                        switch_pct: current_sw as f64 / total as f64,
                        stick_pct: current_stw as f64 / total as f64,
                    });
                    {
                        let mut h = demo_history.write();
                        if h.len() > 50 {
                            let start = h.len() - 50;
                            *h = h[start..].to_vec();
                        }
                    }

                    gloo_timers::future::TimeoutFuture::new((delay as f64 * 1.5) as u32).await;
                }

                is_visual_demo.set(false);
            });
        }
    };

    let mut handle_stop = move |_: ()| {
        is_running.set(false);
        is_visual_demo.set(false);
    };

    // ---- Render ----
    rsx! {
        div { class: "app-container",
            // Header
            header {
                h1 { "Monty Hall Paradox" }
                div { class: "subtitle-row",
                    p { "Experience the counter-intuitive probability puzzle." }
                    div { class: "info-trigger",
                        div { class: "info-icon", "i" }
                        div { class: "info-tooltip",
                            div { class: "info-tooltip-arrow" }
                            span { class: "body-text",
                                span { class: "label", "The Setup:" }
                                "You're on a game show and choose one of three doors: one has a car, two have goats. The host then reveals a goat behind an unchosen door."
                                br {}
                                br {}
                                span { class: "label", "The Question:" }
                                "Should you stick with your original choice, or switch? Play the game to find out!"
                            }
                        }
                    }
                }
            }

            // Main grid
            div { class: "main-grid",
                // Left column
                div { class: "left-col",
                    // Game card
                    div { class: "card game-card",
                        // Overlay when turbo simulation is running
                        if running {
                            div { class: "sim-overlay",
                                div {
                                    div { class: "overlay-text animate-pulse",
                                        "Hyperspace Active... 🚀"
                                        div { class: "overlay-sub",
                                            "Computing millions of universes..."
                                        }
                                    }
                                }
                            }
                        }

                        div { class: "section-title", "Interactive Mode" }

                        // Status line
                        div { class: "status-line",
                            match current_game_state {
                                GameState::Playing => rsx! {
                                    p { class: "status-text choose animate-pulse", "Choose a door!" }
                                },
                                GameState::Revealed => rsx! {
                                    p { class: "status-text switch animate-pulse", "Host revealed a goat! Will you Switch or Stick?" }
                                },
                                GameState::Finished => {
                                    match current_did_win {
                                        Some(true) => rsx! {
                                            p { class: "status-text win", "You won the Car! 🚗🎉" }
                                        },
                                        _ => rsx! {
                                            p { class: "status-text lose", "You got a Goat! 🐐😭" }
                                        },
                                    }
                                }
                            }
                        }

                        // Doors
                        div { class: "doors-row",
                            for door in current_doors.iter() {
                                DoorComponent {
                                    key: "{door.id}",
                                    door: *door,
                                    is_selected: current_player_choice == Some(door.id),
                                    is_host_selected: current_host_choice == Some(door.id),
                                    disabled: current_game_state == GameState::Finished || door.status == DoorStatus::Open || visual_demo || running,
                                    on_click: move |id| handle_door_click(id),
                                }
                            }
                        }

                        // Button row
                        div { class: "button-row",
                            if current_game_state == GameState::Finished && !visual_demo && !running {
                                button {
                                    class: "btn btn-primary",
                                    onclick: move |_| reset_game(()),
                                    "Play Again"
                                }
                                button {
                                    class: "btn btn-secondary",
                                    onclick: move |_| {
                                        interactive_stats.set(Stats::default());
                                        interactive_history.set(vec![]);
                                        reset_game(());
                                    },
                                    "Reset Stats"
                                }
                            }
                        }
                    }

                    // Convergence graph
                    ConvergenceGraph { data: active_history }
                }

                // Right column
                div { class: "right-col",
                    Controls {
                        on_run: move |runs| handle_run_simulation(runs),
                        on_stop: move |_| handle_stop(()),
                        is_running: running,
                        is_visual_demo: visual_demo,
                    }
                    Dashboard {
                        total: active_stats.total,
                        switch_wins: active_stats.switch_wins,
                        stick_wins: active_stats.stick_wins,
                    }
                }
            }

            // Footer
            div { class: "footer-badge",
                "Built with " span { "Rust 🦀" } " + Dioxus + WebAssembly"
            }
        }
    }
}

// ========== Door Component ==========

#[component]
fn DoorComponent(
    door: Door,
    is_selected: bool,
    is_host_selected: bool,
    disabled: bool,
    on_click: EventHandler<usize>,
) -> Element {
    let is_open = door.status == DoorStatus::Open;

    let mut classes = String::from("door");
    if disabled {
        classes.push_str(" disabled");
    }
    if is_selected {
        classes.push_str(" selected");
    }
    if is_open {
        classes.push_str(" open");
    }

    let emoji = if !is_open && !is_selected {
        "🚪"
    } else if !is_open && is_selected {
        "🤔"
    } else if is_open && door.content == DoorContent::Goat {
        "🐐"
    } else {
        "🚗"
    };

    rsx! {
        div {
            class: "{classes}",
            onclick: move |_| {
                if !disabled {
                    on_click.call(door.id);
                }
            },
            div { class: "door-label", "Door {door.id}" }
            div { class: "door-content", "{emoji}" }
            if is_host_selected {
                div { class: "host-badge", "Host Revealed" }
            }
        }
    }
}

// ========== Controls Component ==========

#[component]
fn Controls(
    on_run: EventHandler<u64>,
    on_stop: EventHandler<()>,
    is_running: bool,
    is_visual_demo: bool,
) -> Element {
    let mut runs = use_signal(|| 1_000_000u64);
    let is_active = is_running || is_visual_demo;
    let is_turbo = *runs.read() > 1000;
    let current_runs = *runs.read();

    let btn_class = if is_active {
        "btn-sim stop animate-pulse"
    } else if is_turbo {
        "btn-sim turbo"
    } else {
        "btn-sim demo"
    };

    let btn_label = if is_active {
        "Stop Simulation 🛑"
    } else if is_turbo {
        "Start Turbo Warp Speed ⚡"
    } else {
        "Start Visual Demo 👁\u{fe0f}"
    };

    rsx! {
        div { class: "card controls-card",
            h3 { "Simulation Controls" }
            div { class: "controls-form",
                div {
                    label { "Number of Simulations (≤ 1000 for Visual Demo, > 1000 for Turbo Warp Speed)" }
                    input {
                        r#type: "number",
                        min: "1",
                        max: "10000000000",
                        value: "{current_runs}",
                        disabled: is_active,
                        oninput: move |evt: Event<FormData>| {
                            if let Ok(v) = evt.value().parse::<u64>() {
                                runs.set(v);
                            }
                        },
                    }
                }
                button {
                    class: btn_class,
                    onclick: move |_| {
                        if is_active {
                            on_stop.call(());
                        } else {
                            on_run.call(*runs.read());
                        }
                    },
                    "{btn_label}"
                }
            }
        }
    }
}

// ========== Dashboard Component ==========

#[component]
fn Dashboard(total: u64, switch_wins: u64, stick_wins: u64) -> Element {
    let switch_rate = if total > 0 {
        format!("{:.2}", (switch_wins as f64 / total as f64) * 100.0)
    } else {
        "0.00".to_string()
    };
    let stick_rate = if total > 0 {
        format!("{:.2}", (stick_wins as f64 / total as f64) * 100.0)
    } else {
        "0.00".to_string()
    };

    let total_display = format_compact(total);
    let sw_display = format_compact(switch_wins);
    let stw_display = format_compact(stick_wins);

    rsx! {
        div { class: "card dashboard-card",
            h2 { "Simulation Statistics" }
            div {
                div { class: "stat-total",
                    div { class: "stat-label", "TOTAL GAMES" }
                    div { class: "stat-value", "{total_display}" }
                }

                div { class: "stat-grid",
                    div { class: "stat-card switch-card",
                        div { class: "corner-glow" }
                        div { class: "stat-label", "SWITCH WINS" }
                        div { class: "stat-value", "{switch_rate}%" }
                        div { class: "stat-detail", "({sw_display} wins)" }
                    }
                    div { class: "stat-card stick-card",
                        div { class: "corner-glow" }
                        div { class: "stat-label", "STICK WINS" }
                        div { class: "stat-value", "{stick_rate}%" }
                        div { class: "stat-detail", "({stw_display} wins)" }
                    }
                }
            }
        }
    }
}

// ========== Convergence Graph (Custom SVG via innerHTML) ==========

#[component]
fn ConvergenceGraph(data: Vec<DataPoint>) -> Element {
    if data.is_empty() {
        return rsx! {
            div { class: "card graph-card",
                h3 { "Probability Convergence Over Time" }
                div { class: "graph-container",
                    div { class: "graph-empty", "Waiting for simulation data..." }
                }
            }
        };
    }

    // Chart dimensions (viewBox coordinates)
    let w: f64 = 500.0;
    let h: f64 = 200.0;
    let pad_l: f64 = 45.0;
    let pad_r: f64 = 10.0;
    let pad_t: f64 = 10.0;
    let pad_b: f64 = 25.0;
    let chart_w = w - pad_l - pad_r;
    let chart_h = h - pad_t - pad_b;

    let x_min = data.first().map(|d| d.total as f64).unwrap_or(0.0);
    let x_max = data.last().map(|d| d.total as f64).unwrap_or(1.0);
    let x_range = if (x_max - x_min).abs() < 1.0 { 1.0 } else { x_max - x_min };

    let to_x = |val: f64| -> f64 { pad_l + (val - x_min) / x_range * chart_w };
    let to_y = |pct: f64| -> f64 { pad_t + chart_h - (pct / 100.0) * chart_h };

    // Build polyline points
    let switch_points: String = data
        .iter()
        .map(|d| format!("{:.1},{:.1}", to_x(d.total as f64), to_y(d.switch_pct * 100.0)))
        .collect::<Vec<_>>()
        .join(" ");

    let stick_points: String = data
        .iter()
        .map(|d| format!("{:.1},{:.1}", to_x(d.total as f64), to_y(d.stick_pct * 100.0)))
        .collect::<Vec<_>>()
        .join(" ");

    // Build grid + axis labels
    let mut elements = String::new();
    for val in [0.0f64, 25.0, 50.0, 75.0, 100.0] {
        let y = to_y(val);
        elements.push_str(&format!(
            "<line class=\"chart-grid-line\" x1=\"{}\" y1=\"{:.1}\" x2=\"{}\" y2=\"{:.1}\"/>",
            pad_l, y, w - pad_r, y
        ));
        elements.push_str(&format!(
            "<text class=\"chart-axis-text\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}%</text>",
            pad_l - 5.0, y + 3.0, val as u32
        ));
    }

    // Reference lines
    let ref_y_switch = to_y(66.67);
    let ref_y_stick = to_y(33.33);
    elements.push_str(&format!(
        "<line class=\"chart-ref-line\" stroke=\"#ec4899\" x1=\"{}\" y1=\"{:.1}\" x2=\"{}\" y2=\"{:.1}\"/>",
        pad_l, ref_y_switch, w - pad_r, ref_y_switch
    ));
    elements.push_str(&format!(
        "<line class=\"chart-ref-line\" stroke=\"#06b6d4\" x1=\"{}\" y1=\"{:.1}\" x2=\"{}\" y2=\"{:.1}\"/>",
        pad_l, ref_y_stick, w - pad_r, ref_y_stick
    ));

    // Data polylines
    elements.push_str(&format!(
        "<polyline class=\"chart-line-switch\" points=\"{}\"/>",
        switch_points
    ));
    elements.push_str(&format!(
        "<polyline class=\"chart-line-stick\" points=\"{}\"/>",
        stick_points
    ));

    // X-axis labels
    let first_label = format_compact(data.first().unwrap().total);
    let last_label = format_compact(data.last().unwrap().total);
    elements.push_str(&format!(
        "<text class=\"chart-axis-text\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"start\">{}</text>",
        to_x(x_min), h - 2.0, first_label
    ));
    elements.push_str(&format!(
        "<text class=\"chart-axis-text\" x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\">{}</text>",
        to_x(x_max), h - 2.0, last_label
    ));

    let svg_html = format!(
        "<svg viewBox=\"0 0 {} {}\" preserveAspectRatio=\"xMidYMid meet\" style=\"width:100%;height:100%\">{}</svg>",
        w, h, elements
    );

    rsx! {
        div { class: "card graph-card",
            h3 { "Probability Convergence Over Time" }
            div {
                class: "graph-container",
                dangerous_inner_html: "{svg_html}",
            }
        }
    }
}

// ========== Helpers ==========

fn format_compact(num: u64) -> String {
    if num >= 1_000_000_000 {
        format!("{:.1}B", num as f64 / 1_000_000_000.0)
    } else if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}
