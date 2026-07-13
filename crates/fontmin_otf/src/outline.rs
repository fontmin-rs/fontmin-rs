use allsorts::{
    cff::outline::CFF2Outlines,
    cff::outline::CFFOutlines,
    outline::{OutlineBuilder, OutlineSink},
    pathfinder_geometry::{line_segment::LineSegment2F, vector::Vector2F},
};
use fontmin_diagnostics::{FontminError, Result};

const APPROXIMATION_LIMIT: f64 = 0.292_893_218_813_452_4;
const MAX_SUBDIVISION_DEPTH: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }

    fn midpoint(self, other: Self) -> Self {
        Self {
            x: self.x.midpoint(other.x),
            y: self.y.midpoint(other.y),
        }
    }

    fn length(self) -> f64 {
        self.x.hypot(self.y)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Segment {
    Line(Point),
    Quadratic { control: Point, to: Point },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Contour {
    pub start: Point,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct GlyphPath {
    pub contours: Vec<Contour>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct QuadraticPiece {
    pub from: Point,
    pub control: Point,
    pub to: Point,
    pub t_start: f64,
    pub t_end: f64,
}

#[derive(Debug, Clone, Copy)]
struct Cubic {
    from: Point,
    control1: Point,
    control2: Point,
    to: Point,
    t_start: f64,
    t_end: f64,
}

pub(crate) fn cubic_to_quadratics(
    from: Point,
    control1: Point,
    control2: Point,
    to: Point,
) -> Result<Vec<QuadraticPiece>> {
    let cubic = Cubic {
        from,
        control1,
        control2,
        to,
        t_start: 0.0,
        t_end: 1.0,
    };
    let mut pieces = Vec::new();

    subdivide_cubic(cubic, 0, &mut pieces)?;

    Ok(pieces)
}

pub(crate) fn record_cff_glyph(
    outlines: &mut CFFOutlines<'_, '_>,
    glyph_id: u16,
) -> Result<GlyphPath> {
    let mut recorder = PathRecorder::default();
    outlines
        .visit(glyph_id, None, &mut recorder)
        .map_err(|error| {
            FontminError::invalid_font(format!("invalid CFF glyph {glyph_id}: {error}"))
        })?;

    recorder.finish()
}

pub(crate) fn record_cff2_glyph(
    outlines: &mut CFF2Outlines<'_, '_>,
    glyph_id: u16,
) -> Result<GlyphPath> {
    let mut recorder = PathRecorder::default();
    outlines
        .visit(glyph_id, None, &mut recorder)
        .map_err(|error| {
            FontminError::invalid_font(format!("invalid CFF2 glyph {glyph_id}: {error}"))
        })?;

    recorder.finish()
}

fn subdivide_cubic(cubic: Cubic, depth: u8, pieces: &mut Vec<QuadraticPiece>) -> Result<()> {
    if ![cubic.from, cubic.control1, cubic.control2, cubic.to]
        .into_iter()
        .all(Point::is_finite)
    {
        return Err(FontminError::invalid_font(
            "CFF glyph outline contains a non-finite coordinate",
        ));
    }

    let control = quadratic_control(cubic);
    if cubic_error_bound(cubic) <= APPROXIMATION_LIMIT {
        pieces.push(QuadraticPiece {
            from: cubic.from,
            control,
            to: cubic.to,
            t_start: cubic.t_start,
            t_end: cubic.t_end,
        });
        return Ok(());
    }

    if depth >= MAX_SUBDIVISION_DEPTH {
        return Err(FontminError::invalid_font(
            "CFF cubic outline requires excessive subdivision",
        ));
    }

    let (left, right) = split_cubic(cubic);
    subdivide_cubic(left, depth + 1, pieces)?;
    subdivide_cubic(right, depth + 1, pieces)
}

fn quadratic_control(cubic: Cubic) -> Point {
    Point {
        x: (-cubic.from.x + 3.0 * cubic.control1.x + 3.0 * cubic.control2.x - cubic.to.x) / 4.0,
        y: (-cubic.from.y + 3.0 * cubic.control1.y + 3.0 * cubic.control2.y - cubic.to.y) / 4.0,
    }
}

fn cubic_error_bound(cubic: Cubic) -> f64 {
    Point {
        x: cubic.from.x - 3.0 * cubic.control1.x + 3.0 * cubic.control2.x - cubic.to.x,
        y: cubic.from.y - 3.0 * cubic.control1.y + 3.0 * cubic.control2.y - cubic.to.y,
    }
    .length()
        * 3.0_f64.sqrt()
        / 36.0
}

fn split_cubic(cubic: Cubic) -> (Cubic, Cubic) {
    let left_control = cubic.from.midpoint(cubic.control1);
    let center_control = cubic.control1.midpoint(cubic.control2);
    let right_control = cubic.control2.midpoint(cubic.to);
    let left_quadratic = left_control.midpoint(center_control);
    let right_quadratic = center_control.midpoint(right_control);
    let middle = left_quadratic.midpoint(right_quadratic);
    let middle_t = cubic.t_start.midpoint(cubic.t_end);

    (
        Cubic {
            from: cubic.from,
            control1: left_control,
            control2: left_quadratic,
            to: middle,
            t_start: cubic.t_start,
            t_end: middle_t,
        },
        Cubic {
            from: middle,
            control1: right_quadratic,
            control2: right_control,
            to: cubic.to,
            t_start: middle_t,
            t_end: cubic.t_end,
        },
    )
}

#[derive(Default)]
struct PathRecorder {
    path: GlyphPath,
    current: Option<Contour>,
    error: Option<FontminError>,
}

impl PathRecorder {
    fn finish(mut self) -> Result<GlyphPath> {
        self.finish_contour();

        match self.error {
            Some(error) => Err(error),
            None => Ok(self.path),
        }
    }

    fn finish_contour(&mut self) {
        if let Some(contour) = self.current.take()
            && !contour.segments.is_empty()
        {
            self.path.contours.push(contour);
        }
    }

    fn set_error(&mut self, error: FontminError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    fn current_point(&self) -> Option<Point> {
        let contour = self.current.as_ref()?;

        contour
            .segments
            .last()
            .map_or(Some(contour.start), |segment| {
                Some(match segment {
                    Segment::Line(to) | Segment::Quadratic { to, .. } => *to,
                })
            })
    }

    fn push_line(&mut self, to: Point) {
        if !to.is_finite() {
            self.set_error(FontminError::invalid_font(
                "CFF glyph outline contains a non-finite coordinate",
            ));
            return;
        }

        let Some(contour) = self.current.as_mut() else {
            self.set_error(FontminError::invalid_font(
                "CFF path segment precedes moveTo",
            ));
            return;
        };
        contour.segments.push(Segment::Line(to));
    }

    fn push_quadratic(&mut self, control: Point, to: Point) {
        if !control.is_finite() || !to.is_finite() {
            self.set_error(FontminError::invalid_font(
                "CFF glyph outline contains a non-finite coordinate",
            ));
            return;
        }

        let Some(contour) = self.current.as_mut() else {
            self.set_error(FontminError::invalid_font(
                "CFF path segment precedes moveTo",
            ));
            return;
        };
        contour.segments.push(Segment::Quadratic { control, to });
    }

    fn push_cubic(&mut self, control1: Point, control2: Point, to: Point) {
        let Some(from) = self.current_point() else {
            self.set_error(FontminError::invalid_font(
                "CFF path segment precedes moveTo",
            ));
            return;
        };

        match cubic_to_quadratics(from, control1, control2, to) {
            Ok(pieces) => {
                let Some(contour) = self.current.as_mut() else {
                    return;
                };
                contour
                    .segments
                    .extend(pieces.into_iter().map(|piece| Segment::Quadratic {
                        control: piece.control,
                        to: piece.to,
                    }));
            }
            Err(error) => self.set_error(error),
        }
    }
}

impl OutlineSink for PathRecorder {
    fn move_to(&mut self, to: Vector2F) {
        self.finish_contour();
        let point = Point {
            x: f64::from(to.x()),
            y: f64::from(to.y()),
        };
        if point.is_finite() {
            self.current = Some(Contour {
                start: point,
                segments: Vec::new(),
            });
        } else {
            self.set_error(FontminError::invalid_font(
                "CFF glyph outline contains a non-finite coordinate",
            ));
        }
    }

    fn line_to(&mut self, to: Vector2F) {
        self.push_line(Point {
            x: f64::from(to.x()),
            y: f64::from(to.y()),
        });
    }

    fn quadratic_curve_to(&mut self, control: Vector2F, to: Vector2F) {
        self.push_quadratic(
            Point {
                x: f64::from(control.x()),
                y: f64::from(control.y()),
            },
            Point {
                x: f64::from(to.x()),
                y: f64::from(to.y()),
            },
        );
    }

    fn cubic_curve_to(&mut self, controls: LineSegment2F, to: Vector2F) {
        self.push_cubic(
            Point {
                x: f64::from(controls.from_x()),
                y: f64::from(controls.from_y()),
            },
            Point {
                x: f64::from(controls.to_x()),
                y: f64::from(controls.to_y()),
            },
            Point {
                x: f64::from(to.x()),
                y: f64::from(to.y()),
            },
        );
    }

    fn close(&mut self) {
        self.finish_contour();
    }
}
