use crate::render::PlotItemContext;
use crate::{PlotBounds, PlotPoint, PlotPoints};
use egui::{Color32, Id};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MarkerShape {
    #[default]
    Circle,
    Square,
    Diamond,
    Cross,
}

/// Implement to add custom items. Submit geometry through the context so it
/// participates in the same depth ordering and picking as built-in items.
pub trait PlotItem {
    fn name(&self) -> &str;
    fn id(&self) -> Id {
        Id::new(self.name())
    }
    fn color(&self) -> Option<Color32> {
        None
    }
    fn bounds(&self) -> PlotBounds;
    fn render(&self, context: &mut PlotItemContext<'_>);
    fn allow_hover(&self) -> bool {
        true
    }
}

#[derive(Clone)]
struct Style {
    name: String,
    id: Id,
    color: Option<Color32>,
    highlight: bool,
    hover: bool,
}
impl Style {
    fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: Id::new(&name),
            name,
            color: None,
            highlight: false,
            hover: true,
        }
    }
    fn scale(&self) -> f32 {
        if self.highlight { 1.5 } else { 1.0 }
    }
}
macro_rules! style_methods {
    () => {
        pub fn name(mut self, name: impl ToString) -> Self {
            self.style.name = name.to_string();
            self
        }
        pub fn id(mut self, id: impl egui::AsId) -> Self {
            self.style.id = Id::new(id);
            self
        }
        pub fn color(mut self, color: impl Into<Color32>) -> Self {
            self.style.color = Some(color.into());
            self
        }
        pub fn highlight(mut self, highlight: bool) -> Self {
            self.style.highlight = highlight;
            self
        }
        pub fn allow_hover(mut self, allow: bool) -> Self {
            self.style.hover = allow;
            self
        }
    };
}
macro_rules! trait_style {
    () => {
        fn name(&self) -> &str {
            &self.style.name
        }
        fn id(&self) -> Id {
            self.style.id
        }
        fn color(&self) -> Option<Color32> {
            self.style.color
        }
        fn allow_hover(&self) -> bool {
            self.style.hover
        }
    };
}

/// A cloud of screen-sized markers.
pub struct Points<'a> {
    style: Style,
    series: PlotPoints<'a>,
    radius: f32,
    shape: MarkerShape,
    colors: Option<&'a [Color32]>,
}
impl<'a> Points<'a> {
    pub fn new(name: impl Into<String>, series: impl Into<PlotPoints<'a>>) -> Self {
        Self {
            style: Style::new(name),
            series: series.into(),
            radius: 3.0,
            shape: MarkerShape::Circle,
            colors: None,
        }
    }
    style_methods!();
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.clamp(0.0, 1000.0);
        self
    }
    pub fn shape(mut self, shape: MarkerShape) -> Self {
        self.shape = shape;
        self
    }
    /// Optional per-point colors; missing entries use the item's color.
    pub fn colors(mut self, colors: &'a [Color32]) -> Self {
        self.colors = Some(colors);
        self
    }
}
impl PlotItem for Points<'_> {
    trait_style!();
    fn bounds(&self) -> PlotBounds {
        self.series.bounds()
    }
    fn render(&self, context: &mut PlotItemContext<'_>) {
        context.reserve(self.series.len());
        for (i, &p) in self.series.as_slice().iter().enumerate() {
            let color = self
                .colors
                .and_then(|c| c.get(i))
                .copied()
                .unwrap_or(context.color());
            context.point(
                p.into(),
                self.radius * self.style.scale(),
                self.shape,
                color,
                i,
            );
        }
    }
}

/// A connected polyline. Non-finite samples break the path.
pub struct Line<'a> {
    style: Style,
    series: PlotPoints<'a>,
    width: f32,
}
impl<'a> Line<'a> {
    pub fn new(name: impl Into<String>, series: impl Into<PlotPoints<'a>>) -> Self {
        Self {
            style: Style::new(name),
            series: series.into(),
            width: 1.5,
        }
    }
    style_methods!();
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.clamp(0.0, 1000.0);
        self
    }
}
impl PlotItem for Line<'_> {
    trait_style!();
    fn bounds(&self) -> PlotBounds {
        self.series.bounds()
    }
    fn render(&self, ctx: &mut PlotItemContext<'_>) {
        ctx.polyline(
            self.series.as_slice(),
            self.width * self.style.scale(),
            ctx.color(),
        );
    }
}

pub struct Arrows<'a> {
    style: Style,
    origins: PlotPoints<'a>,
    tips: PlotPoints<'a>,
    width: f32,
    tip_length: f32,
}
impl<'a> Arrows<'a> {
    pub fn new(
        name: impl Into<String>,
        origins: impl Into<PlotPoints<'a>>,
        tips: impl Into<PlotPoints<'a>>,
    ) -> Self {
        Self {
            style: Style::new(name),
            origins: origins.into(),
            tips: tips.into(),
            width: 1.5,
            tip_length: 8.0,
        }
    }
    style_methods!();
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.clamp(0.0, 1000.0);
        self
    }
    pub fn tip_length(mut self, length: f32) -> Self {
        self.tip_length = length.clamp(0.0, 1000.0);
        self
    }
}
impl PlotItem for Arrows<'_> {
    trait_style!();
    fn bounds(&self) -> PlotBounds {
        let mut b = self.origins.bounds();
        b.merge(self.tips.bounds());
        b
    }
    fn render(&self, ctx: &mut PlotItemContext<'_>) {
        for (i, (&a, &b)) in self
            .origins
            .as_slice()
            .iter()
            .zip(self.tips.as_slice())
            .enumerate()
        {
            ctx.arrow(
                a.into(),
                b.into(),
                self.width * self.style.scale(),
                self.tip_length,
                ctx.color(),
                i,
            );
        }
    }
}

/// A filled convex planar polygon. Use separate triangles for non-convex surfaces.
pub struct Polygon<'a> {
    style: Style,
    series: PlotPoints<'a>,
}
impl<'a> Polygon<'a> {
    pub fn new(name: impl Into<String>, series: impl Into<PlotPoints<'a>>) -> Self {
        Self {
            style: Style::new(name),
            series: series.into(),
        }
    }
    style_methods!();
}
impl PlotItem for Polygon<'_> {
    trait_style!();
    fn bounds(&self) -> PlotBounds {
        self.series.bounds()
    }
    fn render(&self, ctx: &mut PlotItemContext<'_>) {
        let p = self.series.as_slice();
        for i in 1..p.len().saturating_sub(1) {
            ctx.triangle([p[0].into(), p[i].into(), p[i + 1].into()], ctx.color());
        }
    }
}

pub struct Text {
    style: Style,
    position: PlotPoint,
    text: String,
}
impl Text {
    pub fn new(
        name: impl Into<String>,
        position: impl Into<PlotPoint>,
        text: impl Into<String>,
    ) -> Self {
        Self {
            style: Style::new(name),
            position: position.into(),
            text: text.into(),
        }
    }
    style_methods!();
}
impl PlotItem for Text {
    trait_style!();
    fn bounds(&self) -> PlotBounds {
        let mut b = PlotBounds::NOTHING;
        b.extend_with(self.position);
        b
    }
    fn render(&self, ctx: &mut PlotItemContext<'_>) {
        ctx.text(self.position, self.text.clone(), ctx.color());
    }
}
