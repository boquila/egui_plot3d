use crate::render::PlotItemContext;
use crate::{PlotBounds, PlotPoint, PlotPoints};
use egui::{Color32, Id};
use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Screen-space marker outline.
pub enum MarkerShape {
    #[default]
    /// Antialiased filled circle (default).
    Circle,
    /// Filled axis-aligned square.
    Square,
    /// Filled diamond.
    Diamond,
    /// Two diagonal strokes.
    Cross,
}

/// Implement to add custom items. Submit geometry through the context so it
/// participates in the same depth ordering and picking as built-in items.
pub trait PlotItem {
    /// Displayed legend/hover name. Empty names omit the legend entry.
    fn name(&self) -> &str;
    /// Stable item ID; defaults to hashing the name.
    fn id(&self) -> Id {
        Id::new(self.name())
    }
    /// Fixed item color, or `None` to use the automatic palette.
    fn color(&self) -> Option<Color32> {
        None
    }
    /// Finite world-space data bounds used for auto-fitting.
    fn bounds(&self) -> PlotBounds;
    /// Submit geometry using the final fitted transform.
    fn render(&self, context: &mut PlotItemContext<'_>);
    /// Whether submitted points and segments can be picked (default true).
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
        /// Change the displayed name without changing the stable item ID.
        pub fn name(mut self, name: impl ToString) -> Self {
            self.style.name = name.to_string();
            self
        }
        /// Override the item ID. Use distinct stable IDs for series sharing a displayed name.
        pub fn id(mut self, id: impl egui::AsId) -> Self {
            self.style.id = Id::new(id);
            self
        }
        /// Set a fixed series color instead of the automatic palette.
        pub fn color(mut self, color: impl Into<Color32>) -> Self {
            self.style.color = Some(color.into());
            self
        }
        /// Enlarge markers or strokes by 1.5x when supported (default false).
        pub fn highlight(mut self, highlight: bool) -> Self {
            self.style.highlight = highlight;
            self
        }
        /// Allow this item to participate in sample picking (default true).
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

/// A filled indexed triangle surface. Vertices can be borrowed, owned, or shared.
///
/// Indices address the supplied vertices. Invalid indices and non-finite faces
/// are skipped; bounds include all supplied vertices. Face colors are optional.
/// Surfaces use average-depth sorting and currently do not participate in picking.
pub struct Surface<'a> {
    style: Style,
    vertices: PlotPoints<'a>,
    indices: Cow<'a, [[u32; 3]]>,
    colors: Option<&'a [Color32]>,
    shading: bool,
}
impl<'a> Surface<'a> {
    /// Create a surface from vertices and triangle index triplets.
    pub fn new(
        name: impl Into<String>,
        vertices: impl Into<PlotPoints<'a>>,
        indices: impl Into<Cow<'a, [[u32; 3]]>>,
    ) -> Self {
        Self {
            style: Style::new(name),
            vertices: vertices.into(),
            indices: indices.into(),
            colors: None,
            shading: false,
        }
    }
    style_methods!();
    /// Per-triangle colors in index order. Missing colors use the item's color.
    pub fn colors(mut self, colors: &'a [Color32]) -> Self {
        self.colors = Some(colors);
        self
    }
    /// Apply two-sided directional lighting, preserving opacity (default false).
    /// Leave disabled when colors encode a quantitative scale.
    pub fn shading(mut self, enabled: bool) -> Self {
        self.shading = enabled;
        self
    }
}
impl PlotItem for Surface<'_> {
    trait_style!();
    fn bounds(&self) -> PlotBounds {
        self.vertices.bounds()
    }
    fn render(&self, context: &mut PlotItemContext<'_>) {
        context.indexed_triangles(
            self.vertices.as_slice(),
            &self.indices,
            self.colors.unwrap_or(&[]),
            self.shading,
        );
    }
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
    /// Create markers from owned, borrowed, or shared coordinates. The name supplies the initial stable item ID.
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
    /// Set marker radius in logical pixels (default 3). Clamped to 0..=1000; zero/NaN markers are not drawn.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius.clamp(0.0, 1000.0);
        self
    }
    /// Choose the marker outline; circles are the default.
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
    /// Create a polyline from owned, borrowed, or shared coordinates. Non-finite samples break the path.
    pub fn new(name: impl Into<String>, series: impl Into<PlotPoints<'a>>) -> Self {
        Self {
            style: Style::new(name),
            series: series.into(),
            width: 1.5,
        }
    }
    style_methods!();
    /// Set stroke width in logical pixels (default 1.5). Clamped to 0..=1000; zero/NaN strokes are not drawn.
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

/// Screen-sized arrowheads connecting pairs of world positions.
pub struct Arrows<'a> {
    style: Style,
    origins: PlotPoints<'a>,
    tips: PlotPoints<'a>,
    width: f32,
    tip_length: f32,
}
impl<'a> Arrows<'a> {
    /// Connect corresponding origins and tips. Extra samples in either sequence are not rendered.
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
    /// Set shaft width in logical pixels (default 1.5); zero/NaN widths suppress the arrow.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.clamp(0.0, 1000.0);
        self
    }
    /// Set head length in logical pixels (default 8), capped at 40% of projected shaft length. Zero/NaN suppresses the head.
    pub fn tip_length(mut self, length: f32) -> Self {
        self.tip_length = length.clamp(0.0, 1000.0);
        self
    }
}
impl PlotItem for Arrows<'_> {
    trait_style!();
    fn bounds(&self) -> PlotBounds {
        let mut b = PlotBounds::NOTHING;
        for (&origin, &tip) in self.origins.as_slice().iter().zip(self.tips.as_slice()) {
            let origin = PlotPoint::from(origin);
            let tip = PlotPoint::from(tip);
            if origin.is_finite() && tip.is_finite() {
                b.extend_with(origin);
                b.extend_with(tip);
            }
        }
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
    /// Fill a convex planar polygon by triangulating its ordered vertices. Fewer than three vertices draw nothing.
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

/// A centered text label anchored to a world-space point.
pub struct Text {
    style: Style,
    position: PlotPoint,
    text: String,
}
impl Text {
    /// Place plain text at a world-space position; its name is used for the legend and item ID.
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
