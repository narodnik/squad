use macroquad::prelude::*;
use macroquad::texture::DrawTextureParams;

const TILE_SIZE: f32 = 20.0;
const GRID_SIZE: i32 = 100;
const ISO_ANGLE_X: f32 = std::f32::consts::FRAC_PI_6;
const ISO_ANGLE_Y: f32 = std::f32::consts::FRAC_PI_6;

/// Audio analysis data - replace these values with real audio analysis
/// when integrating with actual audio input
struct AudioData {
    /// Low frequency (bass) energy - 0.0 to 1.0
    bass: f32,
    /// Mid frequency energy - 0.0 to 1.0
    mid: f32,
    /// High frequency (treble) energy - 0.0 to 1.0
    treble: f32,
    /// Overall volume/energy - 0.0 to 1.0
    volume: f32,
    /// Beat strength - 0.0 (no beat) to 1.0 (strong beat), smoothly interpolated
    beat_strength: f32,
    /// Time for phase tracking - use get_time() or audio position
    time: f32,
}

impl AudioData {
    /// Create mock audio data based on time
    /// Replace this with real audio analysis when ready
    fn from_time(time: f32) -> Self {
        // Smooth beat strength using sine wave for gentle transitions
        let beat_strength = ((time * 4.0).sin() * 0.5 + 0.5).max(0.0).min(1.0);

        Self {
            bass: ((time * 2.0).sin() * 0.5 + 0.5).max(0.0).min(1.0),
            mid: ((time * 3.0).sin() * 0.5 + 0.5).max(0.0).min(1.0),
            treble: ((time * 4.0).sin() * 0.5 + 0.5).max(0.0).min(1.0),
            volume: ((time * 1.5).sin() * 0.5 + 0.5).max(0.0).min(1.0),
            beat_strength,
            time,
        }
    }
}

struct Individual {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    health: i32,
}

impl Individual {
    fn new(x: f32, y: f32) -> Self {
        let angle = rand::gen_range(0.0, std::f32::consts::TAU);
        let speed = 0.2;
        Self {
            x,
            y,
            vx: angle.cos() * speed,
            vy: angle.sin() * speed,
            health: 4,
        }
    }

    fn draw(&self, texture: &Texture2D, center_x: f32, center_y: f32, scale: f32, color: Color, audio: &AudioData) {
        let h = height_at(self.x, self.y, audio);
        let pos = to_isometric(self.x, self.y, h, center_x, center_y);

        let sentinel_size = 4.0;
        let draw_x = pos.x - sentinel_size / 2.0;
        let draw_y = pos.y - sentinel_size;

        draw_texture_ex(
            texture,
            draw_x,
            draw_y,
            color,
            DrawTextureParams {
                dest_size: Some(vec2(
                    texture.width() * scale,
                    texture.height() * scale,
                )),
                ..Default::default()
            },
        );
    }
}

struct Shot {
    x: f32,
    y: f32,
    target_x: f32,
    target_y: f32,
    vx: f32,
    vy: f32,
    active: bool,
    target_swarm: usize,
    target_individual: usize,
}

impl Shot {
    fn new(
        from_x: f32,
        from_y: f32,
        to_x: f32,
        to_y: f32,
        target_swarm: usize,
        target_individual: usize,
    ) -> Self {
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        let dist = (dx * dx + dy * dy).sqrt();
        let speed = 2.0;

        Self {
            x: from_x,
            y: from_y,
            target_x: to_x,
            target_y: to_y,
            vx: dx / dist * speed,
            vy: dy / dist * speed,
            active: true,
            target_swarm,
            target_individual,
        }
    }

    fn update(&mut self) -> bool {
        self.x += self.vx;
        self.y += self.vy;

        let dx = self.x - self.target_x;
        let dy = self.y - self.target_y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist < 1.0 {
            self.active = false;
            return true;
        }
        false
    }

    fn draw(&self, center_x: f32, center_y: f32, audio: &AudioData) {
        if !self.active {
            return;
        }

        let h = height_at(self.x, self.y, audio);
        let pos = to_isometric(self.x, self.y, h, center_x, center_y);

        let size = 20.0;
        draw_rectangle(
            pos.x - size / 2.0,
            pos.y - size / 2.0,
            size,
            size,
            BLACK,
        );
    }
}

struct Swarm {
    individuals: Vec<Individual>,
    color: Color,
}

impl Swarm {
    fn new(count: usize, center_x: f32, center_y: f32, color: Color) -> Self {
        let mut individuals = Vec::new();
        let spread = 10.0;

        for _ in 0..count {
            let offset_x = rand::gen_range(-spread, spread);
            let offset_y = rand::gen_range(-spread, spread);
            individuals.push(Individual::new(
                (center_x + offset_x).clamp(0.0, GRID_SIZE as f32 - 1.0),
                (center_y + offset_y).clamp(0.0, GRID_SIZE as f32 - 1.0),
            ));
        }

        Self { individuals, color }
    }

    fn update(&mut self) {
        const PERCEPTION_RADIUS: f32 = 8.0;
        const SEPARATION_RADIUS: f32 = 2.0;
        const MAX_SPEED: f32 = 0.3;
        const MAX_FORCE: f32 = 0.02;
        const SEPARATION_WEIGHT: f32 = 0.8;
        const ALIGNMENT_WEIGHT: f32 = 0.3;
        const COHESION_WEIGHT: f32 = 0.08;

        let count = self.individuals.len();
        let mut new_velocities = vec![(0.0, 0.0); count];

        for i in 0..count {
            let mut sep_x = 0.0;
            let mut sep_y = 0.0;
            let mut sep_count = 0;

            let mut align_x = 0.0;
            let mut align_y = 0.0;
            let mut align_count = 0;

            let mut coh_x = 0.0;
            let mut coh_y = 0.0;
            let mut coh_count = 0;

            for j in 0..count {
                if i == j {
                    continue;
                }

                let dx = self.individuals[j].x - self.individuals[i].x;
                let dy = self.individuals[j].y - self.individuals[i].y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < PERCEPTION_RADIUS {
                    align_x += self.individuals[j].vx;
                    align_y += self.individuals[j].vy;
                    align_count += 1;

                    coh_x += self.individuals[j].x;
                    coh_y += self.individuals[j].y;
                    coh_count += 1;

                    if dist < SEPARATION_RADIUS && dist > 0.0 {
                        let factor = 1.0 / dist;
                        sep_x -= dx * factor;
                        sep_y -= dy * factor;
                        sep_count += 1;
                    }
                }
            }

            let mut acc_x = 0.0;
            let mut acc_y = 0.0;

            if sep_count > 0 {
                sep_x /= sep_count as f32;
                sep_y /= sep_count as f32;
                let sep_mag = (sep_x * sep_x + sep_y * sep_y).sqrt();
                if sep_mag > 0.0 {
                    sep_x = sep_x / sep_mag * MAX_SPEED - self.individuals[i].vx;
                    sep_y = sep_y / sep_mag * MAX_SPEED - self.individuals[i].vy;
                    let force_mag = (sep_x * sep_x + sep_y * sep_y).sqrt().min(MAX_FORCE);
                    sep_x = sep_x / force_mag * force_mag;
                    sep_y = sep_y / force_mag * force_mag;
                }
                acc_x += sep_x * SEPARATION_WEIGHT;
                acc_y += sep_y * SEPARATION_WEIGHT;
            }

            if align_count > 0 {
                align_x /= align_count as f32;
                align_y /= align_count as f32;
                let align_mag = (align_x * align_x + align_y * align_y).sqrt();
                if align_mag > 0.0 {
                    align_x = align_x / align_mag * MAX_SPEED - self.individuals[i].vx;
                    align_y = align_y / align_mag * MAX_SPEED - self.individuals[i].vy;
                    let force_mag = (align_x * align_x + align_y * align_y).sqrt().min(MAX_FORCE);
                    align_x = align_x / force_mag * force_mag;
                    align_y = align_y / force_mag * force_mag;
                }
                acc_x += align_x * ALIGNMENT_WEIGHT;
                acc_y += align_y * ALIGNMENT_WEIGHT;
            }

            if coh_count > 0 {
                coh_x = coh_x / coh_count as f32 - self.individuals[i].x;
                coh_y = coh_y / coh_count as f32 - self.individuals[i].y;
                let coh_mag = (coh_x * coh_x + coh_y * coh_y).sqrt();
                if coh_mag > 0.0 {
                    coh_x = coh_x / coh_mag * MAX_SPEED - self.individuals[i].vx;
                    coh_y = coh_y / coh_mag * MAX_SPEED - self.individuals[i].vy;
                    let force_mag = (coh_x * coh_x + coh_y * coh_y).sqrt().min(MAX_FORCE);
                    coh_x = coh_x / force_mag * force_mag;
                    coh_y = coh_y / force_mag * force_mag;
                }
                acc_x += coh_x * COHESION_WEIGHT;
                acc_y += coh_y * COHESION_WEIGHT;
            }

            new_velocities[i] = (acc_x, acc_y);
        }

        for i in 0..count {
            self.individuals[i].vx += new_velocities[i].0;
            self.individuals[i].vy += new_velocities[i].1;

            let speed = (self.individuals[i].vx * self.individuals[i].vx
                + self.individuals[i].vy * self.individuals[i].vy)
                .sqrt();
            if speed > MAX_SPEED {
                self.individuals[i].vx = self.individuals[i].vx / speed * MAX_SPEED;
                self.individuals[i].vy = self.individuals[i].vy / speed * MAX_SPEED;
            }

            self.individuals[i].x += self.individuals[i].vx;
            self.individuals[i].y += self.individuals[i].vy;

            let margin = 5.0;
            let turn_factor = 0.02;
            if self.individuals[i].x < margin {
                self.individuals[i].vx += turn_factor;
            }
            if self.individuals[i].x > GRID_SIZE as f32 - margin {
                self.individuals[i].vx -= turn_factor;
            }
            if self.individuals[i].y < margin {
                self.individuals[i].vy += turn_factor;
            }
            if self.individuals[i].y > GRID_SIZE as f32 - margin {
                self.individuals[i].vy -= turn_factor;
            }

            self.individuals[i].x = self.individuals[i].x.clamp(0.0, GRID_SIZE as f32 - 1.0);
            self.individuals[i].y = self.individuals[i].y.clamp(0.0, GRID_SIZE as f32 - 1.0);
        }
    }

    fn draw(&self, texture: &Texture2D, center_x: f32, center_y: f32, scale: f32, audio: &AudioData) {
        for individual in &self.individuals {
            individual.draw(texture, center_x, center_y, scale, self.color, audio);
        }
    }
}

fn height_at(x: f32, y: f32, audio: &AudioData) -> f32 {
    let base_scale1 = 0.08;
    let base_scale2 = 0.15;
    let base_scale3 = 0.25;
    let base_scale4 = 0.40;

    let t = audio.time;

    // Modulate scales with audio frequency bands
    let scale1 = base_scale1 + audio.bass * 0.02;
    let scale2 = base_scale2 + audio.mid * 0.03;
    let scale3 = base_scale3 + audio.treble * 0.04;
    let scale4 = base_scale4 + audio.volume * 0.05;

    // Modulate amplitudes with audio energy - bass affects large waves, treble affects small
    let amp1 = 3.0 + audio.bass * 0.5;
    let amp2 = 2.5 + audio.bass * 0.4;
    let amp3 = 2.0 + audio.mid * 0.3;
    let amp4 = 1.5 + audio.mid * 0.25;
    let amp5 = 1.2 + audio.mid * 0.2;
    let amp6 = 1.8 + audio.treble * 0.25;

    // Smooth beat influence - continuous 0.0 to 1.0 multiplier
    let beat_influence = 1.0 + audio.beat_strength * 0.3;

    let h1 = (x * scale1 + t * 0.5).sin() * amp1 * beat_influence;
    let h2 = (y * scale1 * 0.7 + t * 0.3).cos() * amp2 * beat_influence;
    let h3 = ((x + y) * scale1 * 0.5 + t * 0.4).sin() * amp3;

    let h4 = (x * scale2 + t * 0.6).sin() * amp4;
    let h5 = (y * scale2 + t * 0.2).cos() * amp5;
    let h6 = ((x - y) * scale2 * 0.6 + t * 0.35).cos() * amp6;

    let h7 = (x * scale3 + y * scale3 * 0.4 + t * 0.25).sin() * 0.8;
    let h8 = (y * scale3 * 1.2 + t * 0.4).cos() * 0.6;
    let h9 = ((x + y) * scale3 * 0.8 + t * 0.3).sin() * 0.7;

    let h10 = (x * scale4 + t * 0.15).sin() * 0.3;
    let h11 = (y * scale4 + t * 0.2).cos() * 0.25;
    let h12 = ((x - y * 0.5) * scale4 + t * 0.18).sin() * 0.35;

    h1 + h2 + h3 + h4 + h5 + h6 + h7 + h8 + h9 + h10 + h11 + h12
}

fn get_color(height: f32) -> Color {
    let normalized_height = (height + 15.0) / 30.0;

    if normalized_height < 0.25 {
        let t = normalized_height / 0.25;
        Color {
            r: (10.0 * (1.0 - t) + 34.0 * t) / 255.0,
            g: (50.0 * (1.0 - t) + 139.0 * t) / 255.0,
            b: (10.0 * (1.0 - t) + 34.0 * t) / 255.0,
            a: 1.0,
        }
    } else if normalized_height < 0.45 {
        let t = (normalized_height - 0.25) / 0.2;
        Color {
            r: (34.0 * (1.0 - t) + 76.0 * t) / 255.0,
            g: (139.0 * (1.0 - t) + 175.0 * t) / 255.0,
            b: (34.0 * (1.0 - t) + 80.0 * t) / 255.0,
            a: 1.0,
        }
    } else if normalized_height < 0.65 {
        let t = (normalized_height - 0.45) / 0.2;
        Color {
            r: (76.0 * (1.0 - t) + 154.0 * t) / 255.0,
            g: (175.0 * (1.0 - t) + 205.0 * t) / 255.0,
            b: (80.0 * (1.0 - t) + 50.0 * t) / 255.0,
            a: 1.0,
        }
    } else if normalized_height < 0.85 {
        let t = (normalized_height - 0.65) / 0.2;
        Color {
            r: (154.0 * (1.0 - t) + 200.0 * t) / 255.0,
            g: (205.0 * (1.0 - t) + 230.0 * t) / 255.0,
            b: (50.0 * (1.0 - t) + 200.0 * t) / 255.0,
            a: 1.0,
        }
    } else {
        let t = (normalized_height - 0.85).min(0.15) / 0.15;
        Color {
            r: (200.0 * (1.0 - t) + 255.0 * t) / 255.0,
            g: (230.0 * (1.0 - t) + 255.0 * t) / 255.0,
            b: (200.0 * (1.0 - t) + 255.0 * t) / 255.0,
            a: 1.0,
        }
    }
}

fn to_isometric(x: f32, y: f32, z: f32, center_x: f32, center_y: f32) -> Vec2 {
    let iso_x = (x - y) * ISO_ANGLE_X.cos() * TILE_SIZE + center_x;
    let iso_y = (x + y) * ISO_ANGLE_Y.sin() * TILE_SIZE - z * TILE_SIZE + center_y;
    vec2(iso_x, iso_y)
}

fn conf() -> Conf {
    Conf {
        window_title: "Isometric Landscape".to_owned(),
        platform: miniquad::conf::Platform {
            linux_backend: miniquad::conf::LinuxBackend::WaylandWithX11Fallback,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    rand::srand((macroquad::time::get_time() * 1000000.0) as u64);

    let mut offset_x = 0.0;
    let mut offset_y = -1000.0;
    let pan_speed = 5.0;

    let sentinel_texture: Texture2D = load_texture("sentinel.png").await.unwrap();
    let sprite_scale = 0.15;

    let mut swarms = vec![
        Swarm::new(200, 25.0, 25.0, Color { r: 1.0, g: 0.3, b: 0.3, a: 1.0 }),
        Swarm::new(200, 75.0, 25.0, Color { r: 0.3, g: 1.0, b: 0.3, a: 1.0 }),
        Swarm::new(200, 50.0, 75.0, Color { r: 0.3, g: 0.3, b: 1.0, a: 1.0 }),
    ];

    let mut shots: Vec<Shot> = Vec::new();

    let mut time: f32 = 0.0;

    loop {
        clear_background(BLACK);

        if is_key_down(KeyCode::Left) {
            offset_x += pan_speed;
        }
        if is_key_down(KeyCode::Right) {
            offset_x -= pan_speed;
        }
        if is_key_down(KeyCode::Up) {
            offset_y += pan_speed;
        }
        if is_key_down(KeyCode::Down) {
            offset_y -= pan_speed;
        }

        let center_x = screen_width() / 2.0 + offset_x;
        let center_y = screen_height() / 2.0 + offset_y;

        // Create audio data from current time
        // TODO: Replace this with real audio analysis when ready:
        // let audio_data = AudioData::from_audio(&mut audio_source);
        let audio_data = AudioData::from_time(time);

        for y in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
                let h = height_at(x as f32, y as f32, &audio_data);

                let p1 = to_isometric(x as f32, y as f32, h, center_x, center_y);
                let p2 = to_isometric((x + 1) as f32, y as f32, h, center_x, center_y);
                let p3 = to_isometric((x + 1) as f32, (y + 1) as f32, h, center_x, center_y);
                let p4 = to_isometric(x as f32, (y + 1) as f32, h, center_x, center_y);

                let color = get_color(h);

                draw_triangle(p1, p2, p3, color);
                draw_triangle(p1, p3, p4, color);

                draw_line(p1.x, p1.y, p2.x, p2.y, 1.0, color);
                draw_line(p2.x, p2.y, p3.x, p3.y, 1.0, color);
                draw_line(p3.x, p3.y, p4.x, p4.y, 1.0, color);
                draw_line(p4.x, p4.y, p1.x, p1.y, 1.0, color);
            }
        }

        for swarm in &mut swarms {
            swarm.update();
        }

        const COMBAT_RANGE: f32 = 15.0;
        const SHOT_COOLDOWN: i32 = 30;

        for i in 0..swarms.len() {
            for j in 0..swarms.len() {
                if i == j {
                    continue;
                }

                for shooter in swarms[i].individuals.iter() {
                    for (target_idx, target) in swarms[j].individuals.iter().enumerate() {
                        let dx = target.x - shooter.x;
                        let dy = target.y - shooter.y;
                        let dist = (dx * dx + dy * dy).sqrt();

                        if dist < COMBAT_RANGE {
                            if rand::gen_range(0, SHOT_COOLDOWN) == 0 {
                                shots.push(Shot::new(shooter.x, shooter.y, target.x, target.y, j, target_idx));
                            }
                        }
                    }
                }
            }
        }

        shots.retain(|shot| shot.active);
        for shot in &mut shots {
            if shot.update() {
                if let Some(target_swarm) = swarms.get_mut(shot.target_swarm) {
                    if let Some(target_individual) = target_swarm.individuals.get_mut(shot.target_individual) {
                        target_individual.health -= 1;
                    }
                }
            }
        }

        for swarm in &mut swarms {
            swarm.individuals.retain(|ind| ind.health > 0);
        }

        let alive_swarms = swarms.iter().filter(|s| !s.individuals.is_empty()).count();
        if alive_swarms <= 1 {
            swarms = vec![
                Swarm::new(200, 25.0, 25.0, Color { r: 1.0, g: 0.3, b: 0.3, a: 1.0 }),
                Swarm::new(200, 75.0, 25.0, Color { r: 0.3, g: 1.0, b: 0.3, a: 1.0 }),
                Swarm::new(200, 50.0, 75.0, Color { r: 0.3, g: 0.3, b: 1.0, a: 1.0 }),
            ];
            shots.clear();
            time = 0.0;
        }

        for swarm in &swarms {
            swarm.draw(&sentinel_texture, center_x, center_y, sprite_scale, &audio_data);
        }

        for shot in &shots {
            shot.draw(center_x, center_y, &audio_data);
        }

        draw_text("Isometric Landscape", 10.0, 10.0, 20.0, WHITE);
        draw_text(&format!("Offset: ({:.0}, {:.0})", offset_x, offset_y), 10.0, 35.0, 16.0, WHITE);
        draw_text("Arrow keys to pan", 10.0, 55.0, 16.0, GRAY);

        time += 0.016;

        next_frame().await
    }
}
