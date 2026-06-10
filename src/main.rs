use macroquad::prelude::*;
use macroquad::texture::DrawTextureParams;

const TILE_SIZE: f32 = 20.0;
const GRID_SIZE: i32 = 100;
const ISO_ANGLE_X: f32 = std::f32::consts::FRAC_PI_6;
const ISO_ANGLE_Y: f32 = std::f32::consts::FRAC_PI_6;

struct Individual {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
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
        }
    }

    fn draw(&self, texture: &Texture2D, center_x: f32, center_y: f32, scale: f32, color: Color) {
        let h = height_at(self.x, self.y);
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
                acc_x += sep_x * 0.8;
                acc_y += sep_y * 0.8;
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
                acc_x += align_x * 0.3;
                acc_y += align_y * 0.3;
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
                acc_x += coh_x * 0.15;
                acc_y += coh_y * 0.15;
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

    fn draw(&self, texture: &Texture2D, center_x: f32, center_y: f32, scale: f32) {
        for individual in &self.individuals {
            individual.draw(texture, center_x, center_y, scale, self.color);
        }
    }
}

fn height_at(x: f32, y: f32) -> f32 {
    let scale1 = 0.08;
    let scale2 = 0.15;
    let scale3 = 0.25;
    let scale4 = 0.40;

    let h1 = (x * scale1).sin() * 3.0;
    let h2 = (y * scale1 * 0.7).cos() * 2.5;
    let h3 = ((x + y) * scale1 * 0.5).sin() * 2.0;

    let h4 = (x * scale2).sin() * 1.5;
    let h5 = (y * scale2).cos() * 1.2;
    let h6 = ((x - y) * scale2 * 0.6).cos() * 1.8;

    let h7 = (x * scale3 + y * scale3 * 0.4).sin() * 0.8;
    let h8 = (y * scale3 * 1.2).cos() * 0.6;
    let h9 = ((x + y) * scale3 * 0.8).sin() * 0.7;

    let h10 = (x * scale4).sin() * 0.3;
    let h11 = (y * scale4).cos() * 0.25;
    let h12 = ((x - y * 0.5) * scale4).sin() * 0.35;

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
    let mut offset_x = 0.0;
    let mut offset_y = 0.0;
    let pan_speed = 5.0;

    let sentinel_texture: Texture2D = load_texture("sentinel.png").await.unwrap();
    let sprite_scale = 0.15;

    let mut swarms = vec![
        Swarm::new(40, 25.0, 25.0, Color { r: 1.0, g: 0.3, b: 0.3, a: 1.0 }),
        Swarm::new(40, 75.0, 25.0, Color { r: 0.3, g: 1.0, b: 0.3, a: 1.0 }),
        Swarm::new(40, 50.0, 75.0, Color { r: 0.3, g: 0.3, b: 1.0, a: 1.0 }),
    ];

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

        for y in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
                let h = height_at(x as f32, y as f32);

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

        for swarm in &swarms {
            swarm.draw(&sentinel_texture, center_x, center_y, sprite_scale);
        }

        draw_text("Isometric Landscape", 10.0, 10.0, 20.0, WHITE);
        draw_text(&format!("Offset: ({:.0}, {:.0})", offset_x, offset_y), 10.0, 35.0, 16.0, WHITE);
        draw_text("Arrow keys to pan", 10.0, 55.0, 16.0, GRAY);

        next_frame().await
    }
}
