use fontmin_diagnostics::{FontminError, Result};

use super::{Svgs2TtfOptions, UNITS_PER_EM};

const CURVE_STEPS: u16 = 8;
const ARC_STEPS_PER_TURN: f32 = 32.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Point {
    pub(super) x: i16,
    pub(super) y: i16,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct RawPoint {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Bounds {
    pub(super) x_min: i16,
    pub(super) y_min: i16,
    pub(super) x_max: i16,
    pub(super) y_max: i16,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ViewBox {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) width: f32,
    pub(super) height: f32,
}

#[derive(Debug, Clone, Copy)]
enum PathToken {
    Command(char),
    Number(f32),
}

pub(super) fn numbers(value: &str) -> Vec<f32> {
    tokenize_path(value)
        .into_iter()
        .filter_map(|token| match token {
            PathToken::Number(number) => Some(number),
            PathToken::Command(_) => None,
        })
        .collect()
}

pub(super) fn parse_path_data(path: &str) -> Result<Vec<Vec<RawPoint>>> {
    let tokens = tokenize_path(path);
    let mut parser = PathParser::new(tokens);

    parser.parse()
}

fn tokenize_path(path: &str) -> Vec<PathToken> {
    let bytes = path.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];

        if byte.is_ascii_whitespace() || byte == b',' {
            index += 1;
            continue;
        }
        if byte.is_ascii_alphabetic() {
            tokens.push(PathToken::Command(char::from(byte)));
            index += 1;
            continue;
        }

        let start = index;

        if matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index < bytes.len() && bytes[index] == b'.' {
            index += 1;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
        }
        if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
            let exponent = index;
            index += 1;
            if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
                index += 1;
            }
            let digit_start = index;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                index += 1;
            }
            if digit_start == index {
                index = exponent;
            }
        }

        if start == index {
            index += 1;
            continue;
        }

        if let Ok(number) = path[start..index].parse::<f32>() {
            tokens.push(PathToken::Number(number));
        }
    }

    tokens
}

struct PathParser {
    command: Option<char>,
    contours: Vec<Vec<RawPoint>>,
    current: RawPoint,
    index: usize,
    last_cubic_control: Option<RawPoint>,
    last_quadratic_control: Option<RawPoint>,
    start: RawPoint,
    tokens: Vec<PathToken>,
}

impl PathParser {
    fn new(tokens: Vec<PathToken>) -> Self {
        Self {
            command: None,
            contours: Vec::new(),
            current: RawPoint::default(),
            index: 0,
            last_cubic_control: None,
            last_quadratic_control: None,
            start: RawPoint::default(),
            tokens,
        }
    }

    fn parse(&mut self) -> Result<Vec<Vec<RawPoint>>> {
        let mut contour = Vec::new();

        while self.index < self.tokens.len() {
            if let Some(command) = self.read_command() {
                self.command = Some(command);
            }

            let Some(command) = self.command else {
                return Err(FontminError::invalid_font(
                    "SVG path data must start with a command",
                ));
            };

            match command {
                'M' | 'm' => self.parse_move(command, &mut contour)?,
                'L' | 'l' => self.parse_line(command, &mut contour)?,
                'H' | 'h' => self.parse_horizontal(command, &mut contour)?,
                'V' | 'v' => self.parse_vertical(command, &mut contour)?,
                'Q' | 'q' => self.parse_quadratic(command, &mut contour)?,
                'T' | 't' => self.parse_smooth_quadratic(command, &mut contour)?,
                'C' | 'c' => self.parse_cubic(command, &mut contour)?,
                'S' | 's' => self.parse_smooth_cubic(command, &mut contour)?,
                'A' | 'a' => self.parse_arc(command, &mut contour)?,
                'Z' | 'z' => {
                    close_contour(&mut contour, self.start);
                    push_contour(&mut self.contours, &mut contour);
                    self.current = self.start;
                    self.reset_controls();
                    self.command = None;
                }
                other => {
                    return Err(FontminError::unsupported(format!(
                        "SVG path command {other}",
                    )));
                }
            }
        }

        push_contour(&mut self.contours, &mut contour);

        Ok(std::mem::take(&mut self.contours))
    }

    fn parse_move(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        self.reset_controls();
        let mut first = true;

        while self.has_number() {
            let point = self.read_point(command.is_ascii_lowercase())?;

            if first {
                push_contour(&mut self.contours, contour);
                self.current = point;
                self.start = point;
                contour.push(point);
                first = false;
            } else {
                self.current = point;
                contour.push(point);
            }
        }

        self.command = Some(if command == 'm' { 'l' } else { 'L' });

        Ok(())
    }

    fn parse_line(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        self.reset_controls();
        while self.has_number() {
            let point = self.read_point(command.is_ascii_lowercase())?;

            self.current = point;
            contour.push(point);
        }

        Ok(())
    }

    fn parse_horizontal(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        self.reset_controls();
        while self.has_number() {
            let value = self.read_number()?;
            let x = if command == 'h' {
                self.current.x + value
            } else {
                value
            };

            self.current = RawPoint {
                x,
                y: self.current.y,
            };
            contour.push(self.current);
        }

        Ok(())
    }

    fn parse_vertical(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        self.reset_controls();
        while self.has_number() {
            let value = self.read_number()?;
            let y = if command == 'v' {
                self.current.y + value
            } else {
                value
            };

            self.current = RawPoint {
                x: self.current.x,
                y,
            };
            contour.push(self.current);
        }

        Ok(())
    }

    fn parse_quadratic(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        self.last_cubic_control = None;
        while self.has_number() {
            let control = self.read_point(command.is_ascii_lowercase())?;
            let end = self.read_point(command.is_ascii_lowercase())?;
            let start = self.current;

            for step in 1..=CURVE_STEPS {
                contour.push(quadratic_point(
                    start,
                    control,
                    end,
                    f32::from(step) / f32::from(CURVE_STEPS),
                ));
            }
            self.current = end;
            self.last_quadratic_control = Some(control);
        }

        Ok(())
    }

    fn parse_smooth_quadratic(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        self.last_cubic_control = None;
        while self.has_number() {
            let control = self
                .last_quadratic_control
                .map_or(self.current, |control| reflect(control, self.current));
            let end = self.read_point(command.is_ascii_lowercase())?;
            let start = self.current;

            for step in 1..=CURVE_STEPS {
                contour.push(quadratic_point(
                    start,
                    control,
                    end,
                    f32::from(step) / f32::from(CURVE_STEPS),
                ));
            }
            self.current = end;
            self.last_quadratic_control = Some(control);
        }

        Ok(())
    }

    fn parse_cubic(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        self.last_quadratic_control = None;
        while self.has_number() {
            let first = self.read_point(command.is_ascii_lowercase())?;
            let second = self.read_point(command.is_ascii_lowercase())?;
            let end = self.read_point(command.is_ascii_lowercase())?;
            let start = self.current;

            for step in 1..=CURVE_STEPS {
                contour.push(cubic_point(
                    start,
                    first,
                    second,
                    end,
                    f32::from(step) / f32::from(CURVE_STEPS),
                ));
            }
            self.current = end;
            self.last_cubic_control = Some(second);
        }

        Ok(())
    }

    fn parse_smooth_cubic(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        self.last_quadratic_control = None;
        while self.has_number() {
            let first = self
                .last_cubic_control
                .map_or(self.current, |control| reflect(control, self.current));
            let second = self.read_point(command.is_ascii_lowercase())?;
            let end = self.read_point(command.is_ascii_lowercase())?;
            let start = self.current;

            for step in 1..=CURVE_STEPS {
                contour.push(cubic_point(
                    start,
                    first,
                    second,
                    end,
                    f32::from(step) / f32::from(CURVE_STEPS),
                ));
            }
            self.current = end;
            self.last_cubic_control = Some(second);
        }

        Ok(())
    }

    fn parse_arc(&mut self, command: char, contour: &mut Vec<RawPoint>) -> Result<()> {
        self.reset_controls();
        while self.has_number() {
            let radius_x = self.read_number()?.abs();
            let radius_y = self.read_number()?.abs();
            let rotation = self.read_number()?;
            let large_arc = self.read_arc_flag()?;
            let sweep = self.read_arc_flag()?;
            let mut end = RawPoint {
                x: self.read_number()?,
                y: self.read_number()?,
            };
            if command.is_ascii_lowercase() {
                end.x += self.current.x;
                end.y += self.current.y;
            }

            append_arc(
                contour,
                self.current,
                end,
                radius_x,
                radius_y,
                rotation,
                large_arc,
                sweep,
            );
            self.current = end;
        }

        Ok(())
    }

    fn read_command(&mut self) -> Option<char> {
        let Some(PathToken::Command(command)) = self.tokens.get(self.index).copied() else {
            return None;
        };

        self.index += 1;

        Some(command)
    }

    fn read_number(&mut self) -> Result<f32> {
        let Some(PathToken::Number(number)) = self.tokens.get(self.index).copied() else {
            return Err(FontminError::invalid_font("expected SVG path number"));
        };

        self.index += 1;

        Ok(number)
    }

    fn read_point(&mut self, relative: bool) -> Result<RawPoint> {
        let x = self.read_number()?;
        let y = self.read_number()?;

        if relative {
            Ok(RawPoint {
                x: self.current.x + x,
                y: self.current.y + y,
            })
        } else {
            Ok(RawPoint { x, y })
        }
    }

    fn read_arc_flag(&mut self) -> Result<bool> {
        match self.read_number()? {
            0.0 => Ok(false),
            1.0 => Ok(true),
            _ => Err(FontminError::invalid_font("SVG arc flags must be 0 or 1")),
        }
    }

    fn reset_controls(&mut self) {
        self.last_cubic_control = None;
        self.last_quadratic_control = None;
    }

    fn has_number(&self) -> bool {
        matches!(self.tokens.get(self.index), Some(PathToken::Number(_)))
    }
}

fn reflect(point: RawPoint, around: RawPoint) -> RawPoint {
    RawPoint {
        x: 2.0 * around.x - point.x,
        y: 2.0 * around.y - point.y,
    }
}

#[allow(clippy::too_many_arguments)]
fn append_arc(
    contour: &mut Vec<RawPoint>,
    start: RawPoint,
    end: RawPoint,
    mut radius_x: f32,
    mut radius_y: f32,
    rotation_degrees: f32,
    large_arc: bool,
    sweep: bool,
) {
    if points_equal(start, end) {
        return;
    }
    if radius_x <= f32::EPSILON || radius_y <= f32::EPSILON {
        contour.push(end);
        return;
    }

    let rotation = rotation_degrees
        .to_radians()
        .rem_euclid(std::f32::consts::TAU);
    let (sin_rotation, cos_rotation) = rotation.sin_cos();
    let half_delta = ((start.x - end.x) / 2.0, (start.y - end.y) / 2.0);
    let transformed_x = cos_rotation.mul_add(half_delta.0, sin_rotation * half_delta.1);
    let transformed_y = (-sin_rotation).mul_add(half_delta.0, cos_rotation * half_delta.1);
    let radii_scale =
        transformed_x.powi(2) / radius_x.powi(2) + transformed_y.powi(2) / radius_y.powi(2);
    if radii_scale > 1.0 {
        let scale = radii_scale.sqrt();
        radius_x *= scale;
        radius_y *= scale;
    }

    let numerator = (radius_x * radius_y).powi(2)
        - (radius_x * transformed_y).powi(2)
        - (radius_y * transformed_x).powi(2);
    let denominator = (radius_x * transformed_y).powi(2) + (radius_y * transformed_x).powi(2);
    let center_scale = if denominator <= f32::EPSILON {
        0.0
    } else {
        (numerator.max(0.0) / denominator).sqrt() * if large_arc == sweep { -1.0 } else { 1.0 }
    };
    let transformed_center = RawPoint {
        x: center_scale * radius_x * transformed_y / radius_y,
        y: -center_scale * radius_y * transformed_x / radius_x,
    };
    let center = RawPoint {
        x: cos_rotation.mul_add(transformed_center.x, -sin_rotation * transformed_center.y)
            + start.x.midpoint(end.x),
        y: sin_rotation.mul_add(transformed_center.x, cos_rotation * transformed_center.y)
            + start.y.midpoint(end.y),
    };
    let start_vector = (
        (transformed_x - transformed_center.x) / radius_x,
        (transformed_y - transformed_center.y) / radius_y,
    );
    let end_vector = (
        (-transformed_x - transformed_center.x) / radius_x,
        (-transformed_y - transformed_center.y) / radius_y,
    );
    let start_angle = start_vector.1.atan2(start_vector.0);
    let mut delta_angle = vector_angle(start_vector, end_vector);
    if !sweep && delta_angle > 0.0 {
        delta_angle -= std::f32::consts::TAU;
    } else if sweep && delta_angle < 0.0 {
        delta_angle += std::f32::consts::TAU;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let steps = (delta_angle.abs() / std::f32::consts::TAU * ARC_STEPS_PER_TURN)
        .ceil()
        .max(1.0) as u16;

    for step in 1..=steps {
        let angle = start_angle + delta_angle * f32::from(step) / f32::from(steps);
        let (sin_angle, cos_angle) = angle.sin_cos();
        contour.push(RawPoint {
            x: center.x + cos_rotation * radius_x * cos_angle - sin_rotation * radius_y * sin_angle,
            y: center.y + sin_rotation * radius_x * cos_angle + cos_rotation * radius_y * sin_angle,
        });
    }
    if let Some(last) = contour.last_mut() {
        *last = end;
    }
}

fn vector_angle(first: (f32, f32), second: (f32, f32)) -> f32 {
    let cross = first.0 * second.1 - first.1 * second.0;
    let dot = first.0 * second.0 + first.1 * second.1;

    cross.atan2(dot)
}

fn points_equal(left: RawPoint, right: RawPoint) -> bool {
    (left.x - right.x).abs() <= f32::EPSILON && (left.y - right.y).abs() <= f32::EPSILON
}

fn push_contour(contours: &mut Vec<Vec<RawPoint>>, contour: &mut Vec<RawPoint>) {
    if contour.len() >= 2 {
        contours.push(std::mem::take(contour));
    } else {
        contour.clear();
    }
}

fn close_contour(contour: &mut Vec<RawPoint>, start: RawPoint) {
    if contour.last().is_some_and(|point| {
        (point.x - start.x).abs() > f32::EPSILON || (point.y - start.y).abs() > f32::EPSILON
    }) {
        contour.push(start);
    }
}

fn quadratic_point(start: RawPoint, control: RawPoint, end: RawPoint, t: f32) -> RawPoint {
    let inverse = 1.0 - t;

    RawPoint {
        x: inverse.mul_add(inverse * start.x, 2.0 * inverse * t * control.x) + t * t * end.x,
        y: inverse.mul_add(inverse * start.y, 2.0 * inverse * t * control.y) + t * t * end.y,
    }
}

fn cubic_point(
    start: RawPoint,
    first: RawPoint,
    second: RawPoint,
    end: RawPoint,
    t: f32,
) -> RawPoint {
    let inverse = 1.0 - t;

    RawPoint {
        x: inverse.powi(3) * start.x
            + 3.0 * inverse.powi(2) * t * first.x
            + 3.0 * inverse * t.powi(2) * second.x
            + t.powi(3) * end.x,
        y: inverse.powi(3) * start.y
            + 3.0 * inverse.powi(2) * t * first.y
            + 3.0 * inverse * t.powi(2) * second.y
            + t.powi(3) * end.y,
    }
}

pub(super) fn transform_contour(
    contour: &[RawPoint],
    view_box: ViewBox,
    options: &Svgs2TtfOptions,
) -> Vec<Point> {
    let units = f32::from(UNITS_PER_EM);
    let scale = if options.normalize {
        units / view_box.width.max(view_box.height)
    } else {
        1.0
    };
    let mut points = Vec::with_capacity(contour.len());

    for point in contour {
        let transformed = Point {
            x: clamp_i16(((point.x - view_box.x) * scale).round()),
            y: clamp_i16((f32::from(options.ascent) - (point.y - view_box.y) * scale).round()),
        };

        if points.last().copied() != Some(transformed) {
            points.push(transformed);
        }
    }
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }

    points
}

pub(super) fn transform_font_contour(contour: &[RawPoint], scale: f32) -> Vec<Point> {
    let mut points = Vec::with_capacity(contour.len());

    for point in contour {
        let transformed = Point {
            x: clamp_i16((point.x * scale).round()),
            y: clamp_i16((point.y * scale).round()),
        };

        if points.last().copied() != Some(transformed) {
            points.push(transformed);
        }
    }
    if points.len() > 1 && points.first() == points.last() {
        points.pop();
    }

    points
}

#[allow(clippy::cast_possible_truncation)]
pub(super) fn clamp_i16(value: f32) -> i16 {
    value.clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
pub(super) fn clamp_u16(value: f32) -> u16 {
    value.clamp(0.0, f32::from(u16::MAX)) as u16
}

pub(super) fn bounds_for_contours(contours: &[Vec<Point>]) -> Bounds {
    let mut bounds = Bounds {
        x_min: i16::MAX,
        y_min: i16::MAX,
        x_max: i16::MIN,
        y_max: i16::MIN,
    };

    for point in contours.iter().flatten() {
        bounds.x_min = bounds.x_min.min(point.x);
        bounds.y_min = bounds.y_min.min(point.y);
        bounds.x_max = bounds.x_max.max(point.x);
        bounds.y_max = bounds.y_max.max(point.y);
    }

    bounds
}

#[cfg(test)]
mod tests {
    use super::{CURVE_STEPS, parse_path_data};

    #[test]
    fn parses_smooth_cubic_and_quadratic_commands() {
        let contours =
            parse_path_data("M 0 0 C 10 0 20 10 30 10 S 50 20 60 10 Q 70 0 80 10 T 100 10")
                .unwrap();
        let contour = &contours[0];

        assert_eq!(contour.len(), 1 + usize::from(CURVE_STEPS) * 4);
        assert!((contour.last().unwrap().x - 100.0).abs() < f32::EPSILON);
        assert!((contour.last().unwrap().y - 10.0).abs() < f32::EPSILON);
    }

    #[test]
    fn parses_absolute_and_relative_elliptical_arcs() {
        let contours = parse_path_data("M 10 50 A 40 30 20 0 1 90 50 a 20 20 0 0 0 40 0").unwrap();
        let contour = &contours[0];

        assert!(contour.len() > 4);
        assert!((contour.last().unwrap().x - 130.0).abs() < f32::EPSILON);
        assert!((contour.last().unwrap().y - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rejects_invalid_arc_flags() {
        let error = parse_path_data("M 0 0 A 10 10 0 2 0 20 20").unwrap_err();

        assert!(error.to_string().contains("arc flags must be 0 or 1"));
    }
}
