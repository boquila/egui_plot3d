//! One shared texture keeps circles, antialiased lines, and solid geometry batched.
use egui::{Color32, ColorImage, Context, Id, Pos2, Rect, TextureHandle, TextureOptions};

const SIDE: usize = 256;
const FIRST_ROW: usize = 66;
pub(crate) const SOLID_UV: Pos2 = Pos2::new(32.5 / 256.0, 32.5 / 256.0);
pub(crate) const CIRCLE_UV: Rect = Rect::from_min_max(Pos2::ZERO, Pos2::new(0.25, 0.25));

#[derive(Clone)]
pub(crate) struct Atlas {
    pub texture: TextureHandle,
    widths: Vec<f32>,
}
impl Atlas {
    fn id() -> Id {
        Id::new("egui_plot3d.geometry_atlas.v1")
    }
    pub fn load(ctx: &Context) -> Self {
        ctx.data_mut(|d| d.remove_temp::<Option<Self>>(Self::id()))
            .flatten()
            .unwrap_or_else(|| {
                let mut pixels = vec![Color32::TRANSPARENT; SIDE * SIDE];
                for y in 0..64 {
                    for x in 0..64 {
                        let px = (x as f32 + 0.5) / 64.0 * 2.0 - 1.0;
                        let py = (y as f32 + 0.5) / 64.0 * 2.0 - 1.0;
                        let alpha = ((1.0 - px.hypot(py)) / 0.18).clamp(0.0, 1.0);
                        pixels[y * SIDE + x] =
                            Color32::from_white_alpha((alpha * 255.0).round() as u8);
                    }
                }
                Self {
                    texture: ctx.load_texture(
                        "plot3d geometry",
                        ColorImage::new([SIDE, SIDE], pixels),
                        TextureOptions::LINEAR,
                    ),
                    widths: Vec::new(),
                }
            })
    }
    pub fn store(self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(Self::id(), Some(self)));
    }
    pub fn line_uv(&self, width: f32) -> Option<f32> {
        self.widths
            .iter()
            .position(|&w| w == width)
            .map(|row| (FIRST_ROW as f32 + row as f32 + 0.5) / SIDE as f32)
    }
    pub fn ensure_line(&mut self, width: f32) -> bool {
        if self.line_uv(width).is_some() {
            return true;
        }
        // Very wide strokes need the geometric AA fallback to keep the fringe
        // sharply resolved. A bounded atlas also handles continuously varying widths.
        if !(0.0..=16.0).contains(&width) || self.widths.len() >= SIDE - FIRST_ROW {
            return false;
        }
        let row = FIRST_ROW + self.widths.len();
        self.texture.set_partial(
            [0, row],
            ColorImage::new([SIDE, 1], line_pixels(width)),
            TextureOptions::LINEAR,
        );
        self.widths.push(width);
        true
    }
}
fn line_pixels(width: f32) -> Vec<Color32> {
    let extent = width + 1.4;
    (0..SIDE)
        .map(|i| {
            let x = i as f32 / (SIDE - 1) as f32 * extent;
            let alpha = (x.min(extent - x) / 0.7).clamp(0.0, 1.0);
            Color32::from_white_alpha((alpha * 255.0).round() as u8)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn line_masks_match_the_geometric_antialiasing_profile() {
        for width in [0.1, 0.5, 1.5, 4.0, 16.0] {
            let pixels = line_pixels(width);
            for i in 0..=4096 {
                let t = i as f32 / 4096.0;
                let at = t * (SIDE - 1) as f32;
                let a = at.floor() as usize;
                let b = (a + 1).min(SIDE - 1);
                let fraction = at - a as f32;
                let actual = (pixels[a].a() as f32 * (1.0 - fraction)
                    + pixels[b].a() as f32 * fraction)
                    / 255.0;
                let x = t * (width + 1.4);
                let expected = (x.min(width + 1.4 - x) / 0.7).clamp(0.0, 1.0);
                assert!(
                    (actual - expected).abs() < 0.027,
                    "width {width}, t {t}: {actual} != {expected}"
                );
            }
            assert_eq!(pixels[0], Color32::TRANSPARENT);
            assert_eq!(pixels[SIDE - 1], Color32::TRANSPARENT);
        }
    }
}
