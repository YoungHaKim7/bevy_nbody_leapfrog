use bevy::core_pipeline::core_2d::Camera2dBundle;
use bevy::prelude::*;
use bevy::sprite::SpriteBundle;
use bevy::window::PrimaryWindow;
use rand::{distributions::Standard, rngs::StdRng, Rng, SeedableRng};

const NUM_BODIES: usize = 1000;
const ASPECT_RATIO: f32 = 5.0;

const MAX_X: f32 = 5.0E14;
const MIN_X: f32 = -5.0E14;
const MAX_Y: f32 = 5.0E14;
const MIN_Y: f32 = -5.0E14;

const MAX_MASS: f32 = 9.0E29;
const MIN_MASS: f32 = 1.0E15;

const MAX_V: f32 = 9.0E03;
const MIN_V: f32 = 1.0E03;

const GRAVITATION: f32 = 6.67E-11; // G
const D_TIME: f32 = 2.0E07; // dt (s)
const D_TIME_HALF: f32 = 1.0E07; // dt/2
const A_RIGHT_YEAR: f32 = 9.46E15; // 1 light year (m)

#[derive(Clone, Copy, Debug)]
struct BodyState {
    mass: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    ax: f32,
    ay: f32,
    vx_half: f32,
    vy_half: f32,
    x_new: f32,
    y_new: f32,
    vx_new: f32,
    vy_new: f32,
    ax_new: f32,
    ay_new: f32,
    disp_x: f32, // screen/world mapped
    disp_y: f32,
}
impl BodyState {
    fn new() -> Self {
        Self {
            mass: 0.0,
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            ax: 0.0,
            ay: 0.0,
            vx_half: 0.0,
            vy_half: 0.0,
            x_new: 0.0,
            y_new: 0.0,
            vx_new: 0.0,
            vy_new: 0.0,
            ax_new: 0.0,
            ay_new: 0.0,
            disp_x: 0.0,
            disp_y: 0.0,
        }
    }
}

#[derive(Resource)]
struct Bodies {
    data: Vec<BodyState>,
    elapsed_time: f32,
    kinetic_energy: f64,
    potential_energy: f64,
}

#[derive(Component)]
struct BodyVisual {
    index: usize,
}

#[derive(Component)]
struct UiElapsed;

#[derive(Component)]
struct UiKe;

#[derive(Component)]
struct UiPe;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "(LeapFrog) Star motion by universal gravitation".to_string(),
                resolution: (800., 800.).into(),
                ..Default::default()
            }),
            ..Default::default()
        }))
        .insert_resource(init_bodies())
        .add_systems(Startup, setup)
        .add_systems(Update, (leapfrog_step, update_visuals, update_ui_texts))
        .run();
}

fn init_bodies() -> Bodies {
    let mut rng = StdRng::from_entropy();
    let mut data = vec![BodyState::new(); NUM_BODIES];

    for i in 0..NUM_BODIES {
        let r: f32 = rng.sample(Standard);
        data[i].mass = r * (MAX_MASS - MIN_MASS) + MIN_MASS;

        let r: f32 = rng.sample(Standard);
        data[i].x = r * (MAX_X - MIN_X) + MIN_X;

        let r: f32 = rng.sample(Standard);
        data[i].y = r * (MAX_Y - MIN_Y) + MIN_Y;

        let mut r: f32 = rng.sample(Standard);
        data[i].vx = r * (MAX_V - MIN_V) + MIN_V;
        let flip: f32 = rng.sample(Standard);
        if flip < 0.5 {
            data[i].vx = -data[i].vx;
        }

        r = rng.sample(Standard);
        data[i].vy = r * (MAX_V - MIN_V) + MIN_V;
        let flip: f32 = rng.sample(Standard);
        if flip < 0.5 {
            data[i].vy = -data[i].vy;
        }
    }

    Bodies {
        data,
        elapsed_time: 0.0,
        kinetic_energy: 0.0,
        potential_energy: 0.0,
    }
}

fn setup(mut commands: Commands, bodies: Res<Bodies>, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Tiny white sprites as particles
    for i in 0..NUM_BODIES {
        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: Color::WHITE,
                    custom_size: Some(Vec2::splat(2.0)),
                    ..Default::default()
                },
                transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
                ..Default::default()
            },
            BodyVisual { index: i },
            Name::new(format!("Body {i}")),
        ));
    }

    // UI Text
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let style = TextStyle {
        font,
        font_size: 20.0,
        color: Color::WHITE,
    };

    commands.spawn((
        TextBundle::from_section("elapsed_year: 0.00E+00 year", style.clone())
            .with_text_justify(JustifyText::Left)
            .with_style(Style {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                ..Default::default()
            }),
        UiElapsed,
    ));

    commands.spawn((
        TextBundle::from_section("sum of kinetic energy: 0.00E+00 J", style.clone())
            .with_text_justify(JustifyText::Left)
            .with_style(Style {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(50.0),
                ..Default::default()
            }),
        UiKe,
    ));

    commands.spawn((
        TextBundle::from_section("sum of potential energy: 0.00E+00 J", style)
            .with_text_justify(JustifyText::Left)
            .with_style(Style {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(80.0),
                ..Default::default()
            }),
        UiPe,
    ));

    info!("Initialized {} bodies", bodies.data.len());
}

/// Single-frame Leapfrog: Kick (v^{n+1/2}), Drift (x^{n+1}), Accel, Kick (v^{n+1})
fn leapfrog_step(mut bodies: ResMut<Bodies>) {
    let n = bodies.data.len();

    // Kick: v^{n+1/2} = v^n + a^n * dt/2
    for b in bodies.data.iter_mut() {
        b.vx_half = b.vx + b.ax * D_TIME_HALF;
        b.vy_half = b.vy + b.ay * D_TIME_HALF;
    }

    // Drift: x^{n+1} = x^n + v^{n+1/2} * dt
    for b in bodies.data.iter_mut() {
        b.x_new = b.x + b.vx_half * D_TIME;
        b.y_new = b.y + b.vy_half * D_TIME;
    }

    // Compute a^{n+1} at the drifted positions (O(N^2))
    for i in 0..n {
        bodies.data[i].ax_new = 0.0;
        bodies.data[i].ay_new = 0.0;
    }
    for i in 0..n {
        for j in 0..n {
            if i == j {
                continue;
            }
            let dx = bodies.data[j].x_new - bodies.data[i].x_new;
            let dy = bodies.data[j].y_new - bodies.data[i].y_new;
            let r2 = dx * dx + dy * dy;

            // Ignore very far interactions (>= 1 ly), like your Macroquad version
            let r = r2.sqrt();
            if r > A_RIGHT_YEAR {
                continue;
            }

            // Softening (optional) could go here to avoid singularities; omitted to match original.
            let a_mag = GRAVITATION * bodies.data[j].mass / r2;
            let ax = a_mag * dx / r;
            let ay = a_mag * dy / r;
            bodies.data[i].ax_new += ax;
            bodies.data[i].ay_new += ay;
        }
    }

    // Kick: v^{n+1} = v^{n+1/2} + a^{n+1} * dt/2
    for b in bodies.data.iter_mut() {
        b.vx_new = b.vx_half + b.ax_new * D_TIME_HALF;
        b.vy_new = b.vy_half + b.ay_new * D_TIME_HALF;
    }

    // Advance state (k+1 → k)
    for b in bodies.data.iter_mut() {
        b.x = b.x_new;
        b.y = b.y_new;
        b.vx = b.vx_new;
        b.vy = b.vy_new;
        b.ax = b.ax_new;
        b.ay = b.ay_new;
    }

    // Energies
    // KE = 1/2 m v^2
    let mut ke_sum: f64 = 0.0;
    for b in bodies.data.iter() {
        let v2 = (b.vx * b.vx + b.vy * b.vy) as f64;
        ke_sum += 0.5 * b.mass as f64 * v2;
    }

    // PE = -G \sum_{i<j} m_i m_j / r_ij  (one pass with i<j to avoid double counting)
    let mut pe_sum: f64 = 0.0;
    for i in 0..n {
        for j in (i + 1)..n {
            let dx = (bodies.data[j].x - bodies.data[i].x) as f64;
            let dy = (bodies.data[j].y - bodies.data[i].y) as f64;
            let r = (dx * dx + dy * dy).sqrt();
            if r == 0.0 {
                continue;
            }
            pe_sum +=
                -1.0 * GRAVITATION as f64 * bodies.data[i].mass as f64 * bodies.data[j].mass as f64
                    / r;
        }
    }

    bodies.kinetic_energy = ke_sum;
    bodies.potential_energy = pe_sum;
    bodies.elapsed_time += D_TIME;
}

fn update_visuals(
    mut q: Query<(&BodyVisual, &mut Transform)>,
    mut bodies: ResMut<Bodies>,
    win_q: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = win_q.get_single() else {
        return;
    };
    // Convert space coords → world coords (similar to Macroquad screen mapping)
    let disp_x_conv = window.width() / 2.0 / MAX_X / ASPECT_RATIO;
    let disp_y_conv = window.height() / 2.0 / MAX_Y / ASPECT_RATIO;
    let half_x = window.width() / 2.0;
    let half_y = window.height() / 2.0;

    // Fill disp_x/disp_y fields and move visuals
    for (bv, mut tf) in q.iter_mut() {
        let b = &mut bodies.data[bv.index];
        b.disp_x = b.x * disp_x_conv + half_x;
        b.disp_y = b.y * disp_y_conv + half_y;
        tf.translation.x = b.disp_x - half_x; // center at (0,0) in world
        tf.translation.y = b.disp_y - half_y;
        tf.translation.z = 0.0;
    }
}

fn update_ui_texts(
    bodies: Res<Bodies>,
    mut q_elapsed: Query<&mut Text, With<UiElapsed>>,
    mut q_ke: Query<&mut Text, With<UiKe>>,
    mut q_pe: Query<&mut Text, With<UiPe>>,
) {
    if !bodies.is_changed() {
        return;
    }

    let elapsed_year = bodies.elapsed_time / 3.154E7; // seconds → years
    if let Ok(mut t) = q_elapsed.get_single_mut() {
        t.sections[0].value = format!("elapsed_year:      {:.2E} year", elapsed_year);
    }
    if let Ok(mut t) = q_ke.get_single_mut() {
        t.sections[0].value = format!(
            "sum of kinetic energy:      {:.2E} J",
            bodies.kinetic_energy
        );
    }
    if let Ok(mut t) = q_pe.get_single_mut() {
        t.sections[0].value = format!(
            "sum of potential energy:      {:.2E} J",
            bodies.potential_energy
        );
    }
}
