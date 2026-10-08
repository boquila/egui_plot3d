use crate::colormaps;
use egui::{Align2, Color32, FontId, Mesh, Response, Sense, Shape, Ui, Widget, pos2, vec2};
use std::{ops::RangeInclusive, sync::Arc};

/// Continuous palettes, interpolated in sRGB. Custom stops may include opacity.
#[derive(Clone, Debug)]
pub enum ColorMap {
    /// Purple through blue and green to yellow.
    Viridis,
    /// Black through purple and orange to pale yellow.
    Inferno,
    /// Dark purple through blue and teal to pale green.
    Mako,
    /// Purple through magenta and orange to yellow.
    Plasma,
    /// Evenly spaced custom color stops. Empty palettes are transparent.
    Custom(Arc<[Color32]>),
}
impl ColorMap {
    /// Construct a gradient with evenly spaced stops. One stop is a constant color.
    pub fn custom(colors: impl Into<Arc<[Color32]>>) -> Self {
        Self::Custom(colors.into())
    }
    /// Sample in 0..=1, clamping finite values. Non-finite values are transparent.
    pub fn color(&self, t: f64) -> Color32 {
        if !t.is_finite() {
            return Color32::TRANSPARENT;
        }
        let t = t.clamp(0.0, 1.0);
        if let Self::Custom(stops) = self {
            if stops.is_empty() {
                return Color32::TRANSPARENT;
            }
            let x = t * (stops.len() - 1) as f64;
            let i = x.floor() as usize;
            let a = stops[i].to_array();
            let b = stops[(i + 1).min(stops.len() - 1)].to_array();
            let c: [u8; 4] = std::array::from_fn(|j| {
                (a[j] as f64 + (b[j] as f64 - a[j] as f64) * (x - i as f64)).round() as u8
            });
            return Color32::from_rgba_premultiplied(c[0], c[1], c[2], c[3]);
        }
        let table = match self {
            Self::Viridis => &colormaps::VIRIDIS,
            Self::Inferno => &colormaps::INFERNO,
            Self::Mako => &colormaps::MAKO,
            Self::Plasma => &colormaps::PLASMA,
            Self::Custom(_) => unreachable!(),
        };
        let x = t * 255.0;
        let i = x.floor() as usize;
        let a = table[i];
        let b = table[(i + 1).min(255)];
        let c: [u8; 3] = std::array::from_fn(|j| {
            ((a[j] + (b[j] - a[j]) * (x - i as f64)) * 255.0).round() as u8
        });
        Color32::from_rgb(c[0], c[1], c[2])
    }
}

/// Reusable scalar-to-color mapping shared by plot items and their color key.
///
/// Reversed domains reverse the colors. Constant domains map to the midpoint.
/// Invalid domains and non-finite inputs use the missing color (transparent).
#[derive(Clone, Debug)]
pub struct ColorScale {
    domain: [f64; 2],
    map: ColorMap,
    logarithmic: bool,
    missing: Color32,
}
impl ColorScale {
    /// Map an inclusive numeric domain to a palette.
    pub fn new(domain: RangeInclusive<f64>, map: ColorMap) -> Self {
        Self {
            domain: [*domain.start(), *domain.end()],
            map,
            logarithmic: false,
            missing: Color32::TRANSPARENT,
        }
    }
    /// Use log10 spacing. Nonpositive values/endpoints become missing.
    pub fn logarithmic(mut self) -> Self {
        self.logarithmic = true;
        self
    }
    /// Set the color for missing values or invalid domains.
    pub fn missing_color(mut self, color: Color32) -> Self {
        self.missing = color;
        self
    }
    /// Numeric endpoints, in palette order.
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }
    fn transform(&self, value: f64) -> Option<f64> {
        if !value.is_finite() || (self.logarithmic && value <= 0.0) {
            None
        } else {
            Some(if self.logarithmic {
                value.log10()
            } else {
                value
            })
        }
    }
    /// Map a finite value to 0..=1. Out-of-range values clamp to the palette ends.
    pub fn normalize(&self, value: f64) -> Option<f64> {
        let a = self.transform(self.domain[0])?;
        let b = self.transform(self.domain[1])?;
        let x = self.transform(value)?.clamp(a.min(b), a.max(b));
        if a == b {
            return Some(0.5);
        }
        let span = b - a;
        Some(
            if span.is_finite() {
                (x - a) / span
            } else {
                (x * 0.5 - a * 0.5) / (b * 0.5 - a * 0.5)
            }
            .clamp(0.0, 1.0),
        )
    }
    /// Map a scalar to a color using this scale.
    pub fn color(&self, value: f64) -> Color32 {
        self.normalize(value)
            .map_or(self.missing, |t| self.map.color(t))
    }
    /// Numeric value at a palette position, useful for consistent tick labels.
    pub fn value_at(&self, t: f64) -> Option<f64> {
        if !t.is_finite() {
            return None;
        }
        let a = self.transform(self.domain[0])?;
        let b = self.transform(self.domain[1])?;
        let t = t.clamp(0.0, 1.0);
        if t == 0.0 {
            return Some(self.domain[0]);
        }
        if t == 1.0 {
            return Some(self.domain[1]);
        }
        let x = crate::data::interpolate(a, b, t);
        Some(if self.logarithmic {
            10.0_f64.powf(x)
        } else {
            x
        })
    }
    /// Map a scalar column once, producing owned colors suitable for any item.
    pub fn colors(&self, values: impl IntoIterator<Item = f64>) -> Vec<Color32> {
        values.into_iter().map(|x| self.color(x)).collect()
    }
}

/// An egui widget showing a color scale with its label and numeric ticks.
///
/// `ui.add(ColorBar::new("Temperature", &scale));`
pub struct ColorBar<'a> {
    label: &'a str,
    scale: &'a ColorScale,
    width: f32,
}
impl<'a> ColorBar<'a> {
    /// Create a horizontal scale key, 180 logical pixels wide by default.
    pub fn new(label: &'a str, scale: &'a ColorScale) -> Self {
        Self {
            label,
            scale,
            width: 180.0,
        }
    }
    /// Set width in logical pixels. Invalid widths retain the default.
    pub fn width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.width = width.max(80.0);
        }
        self
    }
}
impl Widget for ColorBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let width = self.width.min(ui.available_width().max(1.0));
        let (rect, response) = ui.allocate_exact_size(vec2(width, 48.0), Sense::hover());
        let painter = ui.painter();
        let text = ui.visuals().text_color();
        let font = FontId::proportional(11.0);
        painter.text(
            rect.left_top(),
            Align2::LEFT_TOP,
            self.label,
            font.clone(),
            text,
        );
        let mut mesh = Mesh::default();
        for i in 0..=64 {
            let t = i as f64 / 64.0;
            let x = rect.left() + rect.width() * t as f32;
            let c = self
                .scale
                .value_at(t)
                .map_or(self.scale.missing, |v| self.scale.color(v));
            mesh.colored_vertex(pos2(x, rect.top() + 17.0), c);
            mesh.colored_vertex(pos2(x, rect.top() + 27.0), c);
            if i > 0 {
                let k = i * 2;
                mesh.add_triangle(k - 2, k - 1, k);
                mesh.add_triangle(k, k - 1, k + 1);
            }
        }
        painter.add(Shape::mesh(mesh));
        for (t, align) in [
            (0.0, Align2::LEFT_TOP),
            (0.5, Align2::CENTER_TOP),
            (1.0, Align2::RIGHT_TOP),
        ] {
            if let Some(v) = self.scale.value_at(t) {
                let label = if v != 0.0 && (v.abs() < 0.001 || v.abs() >= 10000.0) {
                    format!("{v:.1e}")
                } else {
                    format!("{v:.3}")
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_owned()
                };
                painter.text(
                    pos2(rect.left() + t as f32 * rect.width(), rect.top() + 30.0),
                    align,
                    label,
                    font.clone(),
                    text,
                );
            }
        }
        response
    }
}
